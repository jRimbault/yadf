//! Inner parts of `yadf`. Initial file collection and checksumming.

pub mod hash;

use super::TreeBag;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

const BLOCK_SIZE: usize = 4096;

macro_rules! is_match {
    ($regex:expr, $path:expr) => {{
        group($regex.as_ref(), $path.file_name().and_then(|p| p.to_str()))
            .map_or(true, |(regex, name)| regex.is_match(name))
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
    max_depth: Option<usize>,
) -> TreeBag<u64, PathBuf>
where
    H: crate::Hasher,
    P: AsRef<Path>,
{
    let (first, rest) = directories
        .split_first()
        .expect("there should be at least one path");
    let check_entry = |path: &Path, meta: Metadata| {
        !meta.is_file()
            || min.map_or(false, |m| meta.len() < m)
            || max.map_or(false, |m| meta.len() > m)
            || !is_match!(regex, path)
            || !is_match!(glob, path)
    };
    ignore::WalkBuilder::new(first)
        .add_paths(rest)
        .standard_filters(false)
        .max_depth(max_depth)
        .threads(num_cpus::get() / 2)
        .build_parallel()
        .map(|entry| {
            let path = entry.path();
            if check_entry(path, entry.metadata().map_err(|_| ())?) {
                return Err(());
            }
            match hash::partial::<H, _>(&path) {
                Ok(hash) => Ok((hash, path.to_owned())),
                Err(error) => {
                    log::error!("{}, couldn't hash {:?}", error, path);
                    Err(())
                }
            }
        })
        .filter_map(Result::ok)
        .collect()
}

pub(crate) fn dedupe<H: crate::Hasher>(counter: TreeBag<u64, PathBuf>) -> TreeBag<u64, PathBuf> {
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
fn rehash<H: crate::Hasher>(sender: &mpsc::Sender<(u64, PathBuf)>, file: PathBuf, old_hash: u64) {
    if file.metadata().map(|f| f.len()).unwrap_or(0) >= BLOCK_SIZE as _ {
        let hash = match hash::full::<H, _>(&file) {
            Ok(hash) => hash,
            Err(error) => {
                log::error!("{}, couldn't hash {:?}, reusing partial hash", error, file);
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

fn group<T, U>(x: Option<T>, y: Option<U>) -> Option<(T, U)> {
    x.and_then(|r| y.map(|l| (r, l)))
}
