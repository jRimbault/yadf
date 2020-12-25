//! Inner parts of `yadf`. Initial file collection and checksumming.

mod hash;
mod heuristic;

use super::TreeBag;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::fs::Metadata;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

const BLOCK_SIZE: usize = 4096;

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
    H: Hasher + Default,
    P: AsRef<Path>,
{
    let (first, rest) = directories
        .split_first()
        .expect("there should be at least one path");
    let entry_match_criteria = move |path: &Path, meta: Metadata| {
        meta.is_file()
            && min.map_or(true, |m| meta.len() >= m)
            && max.map_or(true, |m| meta.len() <= m)
            && regex.is_file_name_match(path)
            && glob.is_file_name_match(path)
    };
    ignore::WalkBuilder::new(first)
        .add_paths(rest)
        .standard_filters(false)
        .max_depth(max_depth)
        .threads(heuristic::num_cpus_get(directories))
        .build_parallel()
        .map(|entry| {
            let path = entry.path();
            let meta = entry.metadata().map_err(|error| {
                log::error!("{}, couldn't get metadata for {:?}", error, path);
            })?;
            if !entry_match_criteria(path, meta) {
                return Err(());
            }
            let hash = hash::partial::<H, _>(&path).map_err(|error| {
                log::error!("{}, couldn't hash {:?}", error, path);
            })?;
            Ok((hash, path.to_owned()))
        })
        .filter_map(Result::ok)
        .collect()
}

pub(crate) fn dedupe<H>(counter: TreeBag<u64, PathBuf>) -> TreeBag<u64, PathBuf>
where
    H: Hasher + Default,
{
    let (sender, receiver) = crossbeam_channel::unbounded();
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
                        let hash = rehash::<H>(&file).unwrap_or(old_hash);
                        sender.send((hash, file)).unwrap();
                    });
            }
        });
    receiver.into_iter().collect()
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
    fn map<F, I>(self, fnmap: F) -> crossbeam_channel::IntoIter<I>
    where
        F: Fn(ignore::DirEntry) -> I,
        F: Send + Copy,
        I: Send;
}

impl WalkParallelMap for ignore::WalkParallel {
    fn map<F, I>(self, fnmap: F) -> crossbeam_channel::IntoIter<I>
    where
        F: Fn(ignore::DirEntry) -> I,
        F: Send + Copy,
        I: Send,
    {
        let (sender, receiver) = crossbeam_channel::unbounded();
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

trait Matcher {
    fn is_match(&self, text: &str) -> bool;

    #[inline(always)]
    fn is_file_name_match(&self, path: &Path) -> bool {
        match path.file_name().and_then(|p| p.to_str()) {
            Some(file_name) => self.is_match(file_name),
            _ => true,
        }
    }
}

impl Matcher for regex::Regex {
    #[inline(always)]
    fn is_match(&self, text: &str) -> bool {
        regex::Regex::is_match(self, text)
    }
}

impl Matcher for globset::GlobMatcher {
    #[inline(always)]
    fn is_match(&self, text: &str) -> bool {
        globset::GlobMatcher::is_match(self, text)
    }
}

impl<M: Matcher> Matcher for Option<M> {
    #[inline(always)]
    fn is_match(&self, text: &str) -> bool {
        match self {
            Some(matcher) => matcher.is_match(text),
            None => true,
        }
    }

    #[inline(always)]
    fn is_file_name_match(&self, path: &Path) -> bool {
        match self {
            Some(matcher) => matcher.is_file_name_match(path),
            None => true,
        }
    }
}
