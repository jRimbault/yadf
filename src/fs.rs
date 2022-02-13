//! Inner parts of `yadf`. Initial file collection and checksumming.

pub mod filter;
mod hash;
mod heuristic;

use crate::ext::{IteratorExt, WalkBuilderAddPaths, WalkParallelForEach};
use crate::TreeBag;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::hash::Hasher;
use std::path::{Path, PathBuf};

const BLOCK_SIZE: usize = 4096;

/// Foundation of the API.
/// This will attemps a naive scan of every file,
/// within the given size constraints, at the given path.
pub fn find_dupes_partial<H1, H2, P>(
    directories: &[P],
    max_depth: Option<usize>,
    filter: filter::FileFilter,
) -> TreeBag<(u64, u64), PathBuf>
where
    H1: Hasher + Default,
    H2: Hasher + Default,
    P: AsRef<Path>,
{
    let mut paths = directories
        .iter()
        .unique_by(|path| dunce::canonicalize(path).ok());
    let first = paths.next().expect("there should be at least one path");
    let walker = ignore::WalkBuilder::new(first)
        .add_paths(paths)
        .standard_filters(false)
        .max_depth(max_depth)
        .threads(heuristic::num_cpus_get(directories))
        .build_parallel();
    rayon::scope(|scope| {
        let (sender, receiver) = crossbeam_channel::bounded(32);
        scope.spawn(move |_| {
            walker.for_each(|entry| {
                if let Err(error) = entry {
                    log::error!("{}", error);
                    return ignore::WalkState::Continue;
                }
                if let Some(key_value) = hash_entry::<H1, H2>(&filter, entry.unwrap()) {
                    if let Err(error) = sender.send(key_value) {
                        log::error!("{}, couldn't send value across channel", error);
                    }
                }
                ignore::WalkState::Continue
            })
        });
        receiver.into_iter().collect()
    })
}

fn hash_entry<H1, H2>(
    filter: &filter::FileFilter,
    entry: ignore::DirEntry,
) -> Option<((u64, u64), PathBuf)>
where
    H1: Hasher + Default,
    H2: Hasher + Default,
{
    let path = entry.path();
    let meta = entry
        .metadata()
        .map_err(|error| {
            log::error!("{}, couldn't get metadata for {:?}", error, path);
        })
        .ok()?;
    if !filter.is_match(path, meta) {
        return None;
    }
    let hash = hash::partial::<H1, H2, _>(&path)
        .map_err(|error| {
            log::error!("{}, couldn't hash {:?}", error, path);
        })
        .ok()?;
    Some((hash, entry.into_path()))
}

pub fn dedupe<H1, H2>(tree: TreeBag<(u64, u64), PathBuf>) -> crate::FileCounter
where
    H1: Hasher + Default,
    H2: Hasher + Default,
{
    rayon::scope(|scope| {
        let (sender, receiver) = crossbeam_channel::bounded(1024);
        scope.spawn(|_| {
            tree.into_inner()
                .into_par_iter()
                .for_each_with(sender, process_bucket::<H1, H2>)
        });
        receiver.into_iter().collect()
    })
}

fn process_bucket<H1, H2>(
    sender: &mut crossbeam_channel::Sender<((u64, u64), crate::Path)>,
    (old_hash, bucket): ((u64, u64), Vec<PathBuf>),
) where
    H1: Hasher + Default,
    H2: Hasher + Default,
{
    if bucket.len() == 1 {
        let file = bucket.into_iter().next().unwrap();
        if let Err(error) = sender.send((old_hash, file.into())) {
            log::error!("{}, couldn't send value across channel", error);
        }
    } else {
        bucket
            .into_par_iter()
            .for_each_with(sender.clone(), |sender, file| {
                let hash = rehash_file::<H1, H2>(&file).unwrap_or(old_hash);
                if let Err(error) = sender.send((hash, file.into())) {
                    log::error!("{}, couldn't send value across channel", error);
                }
            });
    }
}

fn rehash_file<H1, H2>(file: &Path) -> Result<(u64, u64), ()>
where
    H1: Hasher + Default,
    H2: Hasher + Default,
{
    if file.metadata().map(|f| f.len()).unwrap_or(0) < BLOCK_SIZE as _ {
        return Err(());
    }
    match hash::full::<H1, H2, _>(&file) {
        Ok(hash) => Ok(hash),
        Err(error) => {
            log::error!("{}, couldn't hash {:?}, reusing partial hash", error, file);
            Err(())
        }
    }
}
