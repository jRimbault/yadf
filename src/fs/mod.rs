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
pub fn find_dupes_partial<H, P>(
    directories: &[P],
    max_depth: Option<usize>,
    filter: filter::FileFilter,
) -> TreeBag<u64, PathBuf>
where
    H: Hasher + Default,
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
    let process = |entry: ignore::DirEntry| {
        let path = entry.path();
        let meta = entry.metadata().map_err(|error| {
            log::error!("{}, couldn't get metadata for {:?}", error, path);
        })?;
        if !filter.is_match(path, meta) {
            return Err(());
        }
        let hash = hash::partial::<H, _>(&path).map_err(|error| {
            log::error!("{}, couldn't hash {:?}", error, path);
        })?;
        Ok((hash, path.to_owned()))
    };
    rayon::scope(|scope| {
        let (sender, receiver) = crossbeam_channel::bounded(32);
        scope.spawn(move |_| {
            walker.for_each(|entry| {
                if let Ok(key_value) = process(entry) {
                    sender.send(key_value).unwrap()
                }
            })
        });
        receiver.into_iter().collect()
    })
}

pub fn dedupe<H>(tree: TreeBag<u64, PathBuf>) -> crate::FileCounter
where
    H: Hasher + Default,
{
    rayon::scope(|scope| {
        let (sender, receiver) = crossbeam_channel::bounded(1024);
        scope.spawn(|_| {
            tree.into_inner()
                .into_par_iter()
                .for_each_with(sender, process_bucket::<H>)
        });
        receiver.into_iter().collect()
    })
}

fn process_bucket<H>(
    sender: &mut crossbeam_channel::Sender<(u64, crate::Path)>,
    (old_hash, bucket): (u64, Vec<PathBuf>),
) where
    H: Hasher + Default,
{
    if bucket.len() == 1 {
        let file = bucket.into_iter().next().unwrap();
        sender.send((old_hash, file.into())).unwrap();
    } else {
        bucket
            .into_par_iter()
            .for_each_with(sender.clone(), |sender, file| {
                let hash = rehash_file::<H>(&file).unwrap_or(old_hash);
                sender.send((hash, file.into())).unwrap();
            });
    }
}

fn rehash_file<H>(file: &Path) -> Result<u64, ()>
where
    H: Hasher + Default,
{
    if file.metadata().map(|f| f.len()).unwrap_or(0) < BLOCK_SIZE as _ {
        return Err(());
    }
    match hash::full::<H, _>(&file) {
        Ok(hash) => Ok(hash),
        Err(error) => {
            log::error!("{}, couldn't hash {:?}, reusing partial hash", error, file);
            Err(())
        }
    }
}
