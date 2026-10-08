//! Inner parts of `yadf`. Initial file collection and checksumming.
//!
//! The phases below read as a pipeline over [`TreeBag`]s; the machinery they
//! sit on lives in its own modules: [`pipeline`] for the worker/collector
//! fan-in, [`prefetch`] for cache warming, [`pool`] for the I/O threads,
//! [`file`] and [`hash`] for reading and checksumming.

mod advise;
mod file;
pub mod filter;
mod hash;
mod pipeline;
pub mod pool;
mod prefetch;

use crate::ext::{IteratorExt, WalkBuilderAddPaths};
use crate::path::{Interner, LastDir};
use crate::units::Bytes;
use crate::Factor;
use crate::Path;
use crate::TreeBag;
use pipeline::Sink;
use prefetch::{Progress, Queue, Window};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

/// Files above this size get an extra 4 KiB tail-hash pass before a full
/// read, to cheaply split apart large files that only share a header.
const SUFFIX_HASH_THRESHOLD: Bytes = Bytes::kib(64);

/// A candidate file carried through the hashing pipeline together with its
/// already-known size, so later stages never need to re-`stat` it.
#[derive(Debug)]
pub struct Candidate {
    path: Path,
    size: Bytes,
}

/// Foundation of the API.
///
/// Walks the given paths, groups files by size (a side effect of the
/// metadata the walk already fetches, at no extra syscall cost), then only
/// opens files that share a size with at least one other file: a file with
/// a unique size can never be a duplicate, so it is never read.
///
/// Buckets that can't reach a size `factor` accepts are dropped before
/// being hashed, here and at every later stage.
pub fn find_dupes_partial<H, P>(
    directories: &[P],
    max_depth: Option<usize>,
    filter: filter::FileFilter,
    io_threads: usize,
    factor: Factor,
) -> TreeBag<H::Hash, Candidate>
where
    H: crate::hasher::Hasher,
    P: AsRef<std::path::Path>,
{
    let mut by_size = collect_by_size(directories, max_depth, &filter);
    log::info!(
        "scanned {} files",
        by_size.as_inner().values().map(Vec::len).sum::<usize>()
    );
    by_size.retain_reachable(factor);
    // Only files sharing a size get opened, so only those are worth warming.
    let queue = Queue::covering(&by_size, |path| (path, hash::BLOCK));
    pool::install(io_threads, || {
        queue.warm(Window::PARTIAL, |progress| {
            partial_hash_by_size::<H>(by_size, progress)
        })
    })
}

/// Rehashes every bucket with more than one candidate to confirm (or rule
/// out) a real content match; buckets already known to be unique are
/// passed through untouched.
pub fn dedupe<H>(
    mut tree: TreeBag<H::Hash, Candidate>,
    io_threads: usize,
    factor: Factor,
) -> crate::FileCounter<H::Hash>
where
    H: crate::hasher::Hasher,
{
    tree.retain_reachable(factor);
    let queue = Queue::covering(&tree, |candidate| {
        (&candidate.path, candidate.size.min(prefetch::CONTENT_HEAD))
    });
    pool::install(io_threads, || {
        queue.warm(Window::CONTENT, |progress| {
            pipeline::collect(|sink| {
                tree.into_inner().into_par_iter().for_each_with(
                    sink,
                    |sink, bucket: (H::Hash, Vec<Candidate>)| {
                        let read = bucket.1.len();
                        process_bucket::<H>(sink, bucket, factor);
                        progress.advance(read);
                    },
                )
            })
        })
    })
}

/// Walks `directories` and groups every matching file by its size.
fn collect_by_size<P>(
    directories: &[P],
    max_depth: Option<usize>,
    filter: &filter::FileFilter,
) -> TreeBag<Bytes, Path>
where
    P: AsRef<std::path::Path>,
{
    let mut paths = directories
        .iter()
        .unique_by(|path| dunce::canonicalize(path).ok());
    let first = paths.next().expect("there should be at least one path");
    let walker = ignore::WalkBuilder::new(first)
        .add_paths(paths)
        .standard_filters(false)
        .max_depth(max_depth)
        .threads(num_cpus::get())
        .build_parallel();
    // Every path of the walk hangs off the interned directory nodes, rather
    // than owning a copy of its whole text.
    let dirs = Interner::default();
    pipeline::collect(|sink| {
        let (sink, dirs) = (&sink, &dirs);
        walker.run(|| {
            let mut last_dir = LastDir::default();
            Box::new(move |entry| {
                match entry {
                    Err(error) => log::error!("{}", error),
                    Ok(entry) => {
                        if let Some((size, path)) = size_entry(filter, entry, dirs, &mut last_dir) {
                            sink.send(size, path);
                        }
                    }
                }
                ignore::WalkState::Continue
            })
        })
    })
}

fn size_entry(
    filter: &filter::FileFilter,
    entry: ignore::DirEntry,
    dirs: &Interner,
    last_dir: &mut LastDir,
) -> Option<(Bytes, Path)> {
    let path = entry.path();
    let meta = entry
        .metadata()
        .map_err(|error| log::error!("{}, couldn't get metadata for {:?}", error, path))
        .ok()?;
    let size = Bytes::new(meta.len());
    if !filter.is_match(path, meta) {
        return None;
    }
    Some((size, dirs.path(path, last_dir)))
}

/// Turns size-buckets into partial-hash buckets. Files that are the only
/// one of their size are never opened; the rest are read for their first
/// 4 KiB.
fn partial_hash_by_size<H>(
    by_size: TreeBag<Bytes, Path>,
    progress: &Progress,
) -> TreeBag<H::Hash, Candidate>
where
    H: crate::hasher::Hasher,
{
    pipeline::collect(|sink| {
        by_size.into_inner().into_par_iter().for_each_with(
            sink,
            |sink, bucket: (Bytes, Vec<Path>)| {
                let read = bucket.1.len();
                hash_size_bucket::<H>(sink, bucket);
                progress.advance(read);
            },
        )
    })
}

fn hash_size_bucket<H>(sink: &Sink<H::Hash, Candidate>, (size, bucket): (Bytes, Vec<Path>))
where
    H: crate::hasher::Hasher,
{
    if bucket.len() == 1 {
        let path = bucket.into_iter().next().unwrap();
        sink.send(hash::size_only::<H>(size), Candidate { path, size });
        return;
    }
    bucket
        .into_par_iter()
        .for_each_with(sink.clone(), |sink, path| {
            match path.with_std_path(|std_path| hash::partial::<H>(std_path, size)) {
                Ok(hash) => sink.send(hash, Candidate { path, size }),
                Err(error) => log::error!("{}, couldn't hash {:?}", error, path),
            }
        });
}

fn process_bucket<H>(
    sink: &Sink<H::Hash, crate::Path>,
    (old_hash, bucket): (H::Hash, Vec<Candidate>),
    factor: Factor,
) where
    H: crate::hasher::Hasher,
{
    if bucket.len() == 1 {
        let candidate = bucket.into_iter().next().unwrap();
        sink.send(old_hash, candidate.path);
        return;
    }
    let (large, rest): (Vec<_>, Vec<_>) = bucket
        .into_iter()
        .partition(|candidate| candidate.size >= SUFFIX_HASH_THRESHOLD);

    rest.into_par_iter()
        .for_each_with(sink.clone(), |sink, candidate| {
            match full_hash::<H>(&candidate) {
                HashUpdate::Hash(hash) => sink.send(hash, candidate.path),
                HashUpdate::Verified => sink.send(old_hash, candidate.path),
                HashUpdate::Unreadable => {}
            }
        });

    if large.is_empty() {
        return;
    }
    // A differing tail hash is proof enough that two files differ, no
    // full read needed; only files still colliding on both ends pay for
    // one. Sound because hash *inequality* is exact -- no assumption is
    // being made, unlike the eventual duplicate verdict which (like the
    // rest of yadf) trusts hash equality.
    let by_suffix: TreeBag<H::Hash, Candidate> = large
        .into_par_iter()
        .filter_map(|candidate| match suffix_hash::<H>(&candidate) {
            HashUpdate::Hash(hash) => Some((hash, candidate)),
            HashUpdate::Verified => Some((old_hash, candidate)),
            HashUpdate::Unreadable => None,
        })
        .collect::<Vec<_>>()
        .into_iter()
        .collect();
    by_suffix.into_inner().into_par_iter().for_each_with(
        sink.clone(),
        |sink, (suffix_hash, group)| {
            if !factor.reachable(group.len()) {
                return;
            }
            if group.len() == 1 {
                let candidate = group.into_iter().next().unwrap();
                sink.send(suffix_hash, candidate.path);
                return;
            }
            group
                .into_par_iter()
                .for_each_with(sink.clone(), |sink, candidate| {
                    match full_hash::<H>(&candidate) {
                        HashUpdate::Hash(hash) => sink.send(hash, candidate.path),
                        HashUpdate::Verified => sink.send(suffix_hash, candidate.path),
                        HashUpdate::Unreadable => {}
                    }
                });
        },
    );
}

/// Outcome of trying to refine a candidate's hash by reading more of its
/// content, at either the suffix or full-content stage.
enum HashUpdate<T> {
    /// A freshly-computed hash from this phase.
    Hash(T),
    /// Nothing to gain from reading further -- the caller's current hash
    /// already reflects the whole content. Safe to reuse.
    Verified,
    /// The read failed. The caller's current hash was never confirmed
    /// against this file's actual content and must not be reused; the
    /// candidate is excluded rather than risking a false duplicate.
    Unreadable,
}

/// The candidate's full-content hash, [`HashUpdate::Verified`] if there is
/// nothing to be gained from reading it, or [`HashUpdate::Unreadable`] if
/// the read failed.
fn full_hash<H>(candidate: &Candidate) -> HashUpdate<H::Hash>
where
    H: crate::hasher::Hasher,
{
    if candidate.size < hash::BLOCK {
        // Its partial hash already covered the whole content plus the
        // size: nothing more to distinguish it by.
        return HashUpdate::Verified;
    }
    match candidate
        .path
        .with_std_path(|path| hash::full::<H>(path, candidate.size))
    {
        Ok(hash) => HashUpdate::Hash(hash),
        Err(error) => {
            log::error!(
                "{}, couldn't hash {:?}, excluding it from the results",
                error,
                candidate.path
            );
            HashUpdate::Unreadable
        }
    }
}

/// The candidate's tail hash, or [`HashUpdate::Unreadable`] if it couldn't
/// be read.
fn suffix_hash<H>(candidate: &Candidate) -> HashUpdate<H::Hash>
where
    H: crate::hasher::Hasher,
{
    match candidate
        .path
        .with_std_path(|path| hash::suffix::<H>(path, candidate.size))
    {
        Ok(hash) => HashUpdate::Hash(hash),
        Err(error) => {
            log::error!(
                "{}, couldn't hash suffix of {:?}, excluding it from the results",
                error,
                candidate.path
            );
            HashUpdate::Unreadable
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tempdir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("yadf-fs-test-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn full_hash_is_verified_below_block_size() {
        let dir = tempdir("verified");
        let path = dir.join("small");
        std::fs::write(&path, b"tiny").unwrap();
        let size = Bytes::new(std::fs::metadata(&path).unwrap().len());
        assert!(size < hash::BLOCK);
        let candidate = Candidate {
            path: Path::from_path(&path),
            size,
        };
        assert!(matches!(
            full_hash::<seahash::SeaHasher>(&candidate),
            HashUpdate::Verified
        ));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn full_hash_is_unreadable_for_a_missing_file() {
        let candidate = Candidate {
            path: Path::from_path(&tempdir("full-missing").join("does-not-exist")),
            size: hash::BLOCK,
        };
        assert!(matches!(
            full_hash::<seahash::SeaHasher>(&candidate),
            HashUpdate::Unreadable
        ));
    }

    #[test]
    fn suffix_hash_is_unreadable_for_a_missing_file() {
        let candidate = Candidate {
            path: Path::from_path(&tempdir("suffix-missing").join("does-not-exist-either")),
            size: SUFFIX_HASH_THRESHOLD,
        };
        assert!(matches!(
            suffix_hash::<seahash::SeaHasher>(&candidate),
            HashUpdate::Unreadable
        ));
    }

    /// Reproduces the original bug: two files sharing a partial hash, one
    /// of which disappears before the content-hashing phase reads it. The
    /// disappeared file must be excluded, never merged into the surviving
    /// file's group under the stale partial hash.
    #[test]
    fn unreadable_candidate_is_excluded_not_merged_under_the_old_hash() {
        let dir = tempdir("carryover");
        let content = vec![b'a'; 8192];

        let survivor_path = dir.join("survivor");
        std::fs::write(&survivor_path, &content).unwrap();
        let size = Bytes::new(std::fs::metadata(&survivor_path).unwrap().len());

        let vanished_path = dir.join("vanished");
        std::fs::write(&vanished_path, &content).unwrap();
        // Simulate a read failure landing between the partial-hash phase
        // (which already grouped these under one key) and the
        // content-hashing phase.
        std::fs::remove_file(&vanished_path).unwrap();

        let old_hash = hash::partial::<seahash::SeaHasher>(&survivor_path, size).unwrap();
        let bucket = vec![
            Candidate {
                path: Path::from_path(&survivor_path),
                size,
            },
            Candidate {
                path: Path::from_path(&vanished_path),
                size,
            },
        ];

        let result: TreeBag<u64, crate::Path> = pipeline::collect(|sink| {
            process_bucket::<seahash::SeaHasher>(&sink, (old_hash, bucket), Factor::default());
        });

        let paths: Vec<_> = result
            .into_inner()
            .into_values()
            .flatten()
            .map(|path| path.to_path_buf())
            .collect();
        assert_eq!(paths, vec![survivor_path]);

        std::fs::remove_dir_all(dir).unwrap();
    }
}
