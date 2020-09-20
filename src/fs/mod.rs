//! Inner parts of `yadf`. Initial file collection and checksumming.

pub mod hash;
#[cfg(not(tarpaulin_include))]
pub(crate) mod wrapper;

use super::TreeBag;
use hash::FsHasher;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::fs;
use std::hash::Hasher;
use std::path::Path;
use std::sync::mpsc;
use wrapper::DirEntry;

const BLOCK_SIZE: usize = 4096;

/// Foundation of the API.
/// This will attemps a naive scan of every file,
/// within the given size constraints, at the given path.
pub(crate) fn find_dupes_partial<H, P>(
    dir: &[P],
    min: Option<u64>,
    max: Option<u64>,
) -> TreeBag<u64, DirEntry>
where
    H: Hasher + Default,
    P: AsRef<Path>,
{
    let (first, rest) = dir.split_first().unwrap();
    ignore::WalkBuilder::new(first)
        .add_paths(rest.into_iter())
        .standard_filters(false)
        .threads(num_cpus::get())
        .build_parallel()
        .map(|entry| {
            let meta = fs::symlink_metadata(entry.path()).map_err(|_| ())?;
            if !meta.is_file() {
                return Err(());
            }
            if min.map_or(false, |m| meta.len() < m) {
                return Err(());
            }
            if max.map_or(false, |m| meta.len() > m) {
                return Err(());
            }
            let hasher: FsHasher<H> = Default::default();
            if let Ok(hash) = hasher.partial(entry.path()) {
                return Ok((hash, DirEntry(entry)));
            }
            Err(())
        })
        .filter_map(Result::ok)
        .collect()
}

pub(crate) fn dedupe<H>(counter: TreeBag<u64, DirEntry>) -> TreeBag<u64, DirEntry>
where
    H: Hasher + Default,
{
    let (sender, receiver) = mpsc::channel();
    counter
        .0
        .into_par_iter()
        .for_each_with(sender, |sender, (old_hash, bucket)| {
            if bucket.len() == 1 {
                let file = bucket.into_iter().next().unwrap();
                sender.send((old_hash, file)).unwrap();
            } else {
                bucket
                    .into_par_iter()
                    .for_each_with(sender.clone(), |sender, file| {
                        if file.metadata().map(|f| f.len()).unwrap_or(0) >= BLOCK_SIZE as _ {
                            let hasher: FsHasher<H> = Default::default();
                            let hash = hasher.full(file.path()).unwrap_or(old_hash);
                            sender.send((hash, file)).unwrap();
                        } else {
                            sender.send((old_hash, file)).unwrap();
                        }
                    });
            }
        });
    receiver.into_iter().collect()
}

trait WalkParallelMap {
    fn map<F, I>(self, fnmap: F) -> mpsc::IntoIter<I>
    where
        F: Fn(ignore::DirEntry) -> I,
        F: Send + Copy,
        I: Send;
}

impl WalkParallelMap for ignore::WalkParallel {
    fn map<F, I>(self, fnmap: F) -> mpsc::IntoIter<I>
    where
        F: Fn(ignore::DirEntry) -> I,
        F: Send + Copy,
        I: Send,
    {
        let (sender, receiver) = mpsc::channel();
        self.run(move || {
            let sender = sender.clone();
            Box::new(move |result| {
                if let Ok(entry) = result {
                    sender.send(fnmap(entry)).unwrap();
                }
                ignore::WalkState::Continue
            })
        });
        receiver.into_iter()
    }
}

trait WalkBuilderAddPaths {
    fn add_paths<P, I>(&mut self, paths: I) -> &mut Self
    where
        P: AsRef<Path>,
        I: Iterator<Item = P>;
}

impl WalkBuilderAddPaths for ignore::WalkBuilder {
    fn add_paths<P, I>(&mut self, paths: I) -> &mut Self
    where
        P: AsRef<Path>,
        I: Iterator<Item = P>,
    {
        for path in paths {
            self.add(path);
        }
        self
    }
}
