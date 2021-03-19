//! Inner parts of `yadf`. Initial file collection and checksumming.

pub mod filter;
mod hash;
mod heuristic;

use super::TreeBag;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::{Path, PathBuf};
use std::{collections::HashSet, hash::Hasher};

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
    let mut paths = unique_paths(directories);
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
        scope.spawn(move |_| walker.for_each(|entry| sender.send(process(entry)).unwrap()));
        receiver.into_iter().filter_map(Result::ok).collect()
    })
}

pub fn dedupe<H>(bag: TreeBag<u64, PathBuf>) -> crate::FileCounter
where
    H: Hasher + Default,
{
    rayon::scope(|scope| {
        let (sender, receiver) = crossbeam_channel::bounded(32);
        scope.spawn(|_| {
            bag.into_inner()
                .into_par_iter()
                .for_each(move |(old_hash, bucket)| {
                    if bucket.len() == 1 {
                        let file = bucket.into_iter().next().unwrap();
                        sender.send((old_hash, file.into())).unwrap();
                    } else {
                        bucket
                            .into_par_iter()
                            .for_each_with(sender.clone(), |sender, file| {
                                let hash = rehash::<H>(&file).unwrap_or(old_hash);
                                sender.send((hash, file.into())).unwrap();
                            });
                    }
                });
        });
        receiver.into_iter().collect()
    })
}

// decrease indent level of the dedupe function
fn rehash<H>(file: &Path) -> Result<u64, ()>
where
    H: Hasher + Default,
{
    if file.metadata().map(|f| f.len()).unwrap_or(0) >= BLOCK_SIZE as _ {
        match hash::full::<H, _>(&file) {
            Ok(hash) => Ok(hash),
            Err(error) => {
                log::error!("{}, couldn't hash {:?}, reusing partial hash", error, file);
                Err(())
            }
        }
    } else {
        Err(())
    }
}

trait WalkParallelMap {
    fn for_each<F>(self, f: F)
    where
        F: Fn(ignore::DirEntry),
        F: Send + Copy;
}

impl WalkParallelMap for ignore::WalkParallel {
    fn for_each<F>(self, f: F)
    where
        F: Fn(ignore::DirEntry),
        F: Send + Copy,
    {
        self.run(move || {
            Box::new(move |result| {
                match result {
                    Ok(entry) => f(entry),
                    Err(error) => log::error!("{}", error),
                }
                ignore::WalkState::Continue
            })
        })
    }
}

trait WalkBuilderAddPaths {
    fn add_paths<P, I>(&mut self, paths: I) -> &mut Self
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>;
}

impl WalkBuilderAddPaths for ignore::WalkBuilder {
    fn add_paths<P, I>(&mut self, paths: I) -> &mut Self
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>,
    {
        for path in paths {
            self.add(path);
        }
        self
    }
}

fn unique_paths<P, I>(paths: I) -> impl Iterator<Item = P>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = P>,
{
    let mut paths_set = HashSet::new();
    paths.into_iter().filter_map(move |path| {
        if paths_set.insert(dunce::canonicalize(&path).ok()?) {
            Some(path)
        } else {
            None
        }
    })
}
