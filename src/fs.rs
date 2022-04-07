//! Inner parts of `yadf`. Initial file collection and checksumming.

pub mod filter;
mod hash;
mod heuristic;

use crate::ext::{IteratorExt, WalkBuilderAddPaths};
use crate::TreeBag;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::{
    hash::Hasher,
    path::{Path, PathBuf},
};

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
        .build();
    let num_cpus = num_cpus::get();
    rayon::scope(|scope| {
        let (send_hashed_entry, hashed_entries) = crossbeam_channel::bounded(32);
        let (to_hash, entries_to_hash) = crossbeam_channel::bounded(32);
        let filter = &filter;
        scope.spawn(move |_| {
            for entry in walker {
                match entry {
                    Err(error) => log::error!("{}", error),
                    Ok(entry) => to_hash.send(entry).unwrap(),
                }
            }
        });
        for _ in 0..num_cpus {
            let entries_to_hash = entries_to_hash.clone();
            let send_hashed_entry = send_hashed_entry.clone();
            scope.spawn(move |_| {
                for entry in entries_to_hash {
                    if let Some(key_value) = hash_entry::<H>(filter, entry) {
                        if let Err(error) = send_hashed_entry.send(key_value) {
                            log::error!("{}, couldn't send value across channel", error);
                        }
                    }
                }
            });
        }
        hashed_entries.into_iter().collect()
    })
}

fn hash_entry<H>(filter: &filter::FileFilter, entry: ignore::DirEntry) -> Option<(u64, PathBuf)>
where
    H: Hasher + Default,
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
    let hash = hash::partial::<H, _>(&path)
        .map_err(|error| {
            log::error!("{}, couldn't hash {:?}", error, path);
        })
        .ok()?;
    Some((hash, entry.into_path()))
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
        if let Err(error) = sender.send((old_hash, file.into())) {
            log::error!("{}, couldn't send value across channel", error);
        }
    } else {
        bucket
            .into_par_iter()
            .for_each_with(sender.clone(), |sender, file| {
                let hash = rehash_file::<H>(&file).unwrap_or(old_hash);
                if let Err(error) = sender.send((hash, file.into())) {
                    log::error!("{}, couldn't send value across channel", error);
                }
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
