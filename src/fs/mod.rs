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
pub(crate) fn find_dupes_partial<H>(
    dir: &Path,
    min_max_file_size: Option<(u64, u64)>,
) -> TreeBag<u64, DirEntry>
where
    H: Hasher + Default,
{
    let (sender, receiver) = mpsc::channel();
    let (min, max) = min_max_file_size.unwrap_or((0, u64::MAX));
    ignore::WalkBuilder::new(dir)
        .standard_filters(false)
        .build_parallel()
        // move sender into closure
        .run(move || {
            let sender = sender.clone();
            Box::new(move |result| {
                if result.is_err() {
                    return ignore::WalkState::Continue;
                }
                let entry = result.unwrap();
                let meta = fs::symlink_metadata(entry.path());
                if meta.is_err() {
                    return ignore::WalkState::Continue;
                }
                let meta = meta.unwrap();
                let len = meta.len();
                if meta.is_file() && len >= min && len <= max {
                    let hasher: FsHasher<H> = Default::default();
                    if let Ok(hash) = hasher.partial(entry.path()) {
                        sender.send((hash, DirEntry(entry))).unwrap();
                    }
                }
                ignore::WalkState::Continue
            })
        });
    receiver.into_iter().collect()
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
