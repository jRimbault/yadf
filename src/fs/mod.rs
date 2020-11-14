//! Inner parts of `yadf`. Initial file collection and checksumming.

pub mod hash;
#[cfg(not(tarpaulin_include))]
pub(crate) mod wrapper;

use super::TreeBag;
use hash::FileHasher;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::fs;
use std::path::Path;
use std::sync::mpsc;
use wrapper::DirEntry;

const BLOCK_SIZE: usize = 4096;

macro_rules! is_match {
    ($regex:expr, $entry:expr) => {{
        $regex
            .as_ref()
            .and_then(|r| $entry.path().file_name().map(|n| (r, n)))
            .map_or(true, |(regex, name)| {
                regex.is_match(name.to_string_lossy().as_ref())
            })
    }};
}

/// Foundation of the API.
/// This will attemps a naive scan of every file,
/// within the given size constraints, at the given path.
pub(crate) fn find_dupes_partial<H, P>(
    directories: &[P],
    min: Option<u64>,
    max: Option<u64>,
    regex: Option<regex::Regex>,
    glob: Option<globset::GlobMatcher>,
) -> TreeBag<u64, DirEntry>
where
    H: crate::Hasher,
    P: AsRef<Path>,
{
    let (first, rest) = directories.split_first().unwrap();
    ignore::WalkBuilder::new(first)
        .add_paths(rest.iter())
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
            if !is_match!(regex, entry) {
                return Err(());
            }
            if !is_match!(glob, entry) {
                return Err(());
            }
            let hash = match FileHasher::<H>::partial(&entry.path()) {
                Ok(hash) => hash,
                Err(error) => {
                    log::error!("{}, couldn't hash {:?}", error, entry.path());
                    return Err(());
                }
            };
            Ok((hash, DirEntry(entry)))
        })
        .filter_map(Result::ok)
        .collect()
}

pub(crate) fn dedupe<H: crate::Hasher>(counter: TreeBag<u64, DirEntry>) -> TreeBag<u64, DirEntry> {
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
                        rehash::<H>(sender, file, old_hash)
                    });
            }
        });
    receiver.into_iter().collect()
}

// decrease indent level of the dedupe function
fn rehash<H: crate::Hasher>(
    sender: &mut mpsc::Sender<(u64, DirEntry)>,
    file: DirEntry,
    old_hash: u64,
) {
    if file.metadata().map(|f| f.len()).unwrap_or(0) >= BLOCK_SIZE as _ {
        let hash = match FileHasher::<H>::full(&file.path()) {
            Ok(hash) => hash,
            Err(error) => {
                log::error!(
                    "{}, couldn't hash {:?}, reusing partial hash",
                    error,
                    file.path()
                );
                old_hash
            }
        };
        sender.send((hash, file)).unwrap();
    } else {
        sender.send((old_hash, file)).unwrap();
    }
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
                match result {
                    Ok(entry) => sender.send(fnmap(entry)).unwrap(),
                    Err(error) => log::error!("{}", error),
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
