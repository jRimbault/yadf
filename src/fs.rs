//! Inner parts of `yadf`. Initial file collection and checksumming.

mod advise;
pub mod filter;
mod hash;

use crate::ext::{IteratorExt, WalkBuilderAddPaths, WalkParallelForEach};
use crate::TreeBag;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const CHANNEL_SIZE: usize = 8 * 1024;
const BLOCK_SIZE: usize = 4 * 1024;
/// Files above this size get an extra 4 KiB tail-hash pass before a full
/// read, to cheaply split apart large files that only share a header.
const SUFFIX_HASH_THRESHOLD: u64 = 64 * 1024;

/// Threads dedicated to issuing readahead hints. They only ever `open` and
/// `posix_fadvise`, never read or hash, so they cost almost no CPU and can
/// safely outnumber the cores.
const PREFETCH_THREADS: usize = 32;
/// How far ahead of the hashing threads the prefetcher is allowed to run,
/// in files. Bounds how much speculative data can sit in the page cache.
const PARTIAL_PREFETCH_WINDOW: usize = 4096;
const CONTENT_PREFETCH_WINDOW: usize = 256;
/// Only the head of a file is prefetched in the content phase; once the
/// read is under way `POSIX_FADV_SEQUENTIAL` keeps it fed. Prefetching whole
/// multi-hundred-MB files would evict more than it gains.
const CONTENT_PREFETCH_LEN: u64 = 1024 * 1024;

/// Default concurrency for the I/O-bound hashing phases, distinct from (but
/// currently equal to) the walker's concurrency.
///
/// Oversubscribing well past the core count can help small random reads on
/// some SSD/NVMe devices saturate their queue depth (see pkolaczk's
/// disk-parallelism measurements), but it is not a safe default: measured
/// locally, going from 1x to 4x cores bought a few percent on a cold cache
/// while costing up to 60% on a warm one, because the extra threads have
/// nothing but each other to contend with once there is no device latency
/// to hide. Use `--io-threads` to opt into oversubscription on storage
/// where it is known to help.
pub fn default_io_threads() -> usize {
    num_cpus::get()
}

/// A candidate file carried through the hashing pipeline together with its
/// already-known size, so later stages never need to re-`stat` it.
#[derive(Debug)]
pub struct Candidate {
    path: PathBuf,
    size: u64,
}

impl AsRef<Path> for Candidate {
    fn as_ref(&self) -> &Path {
        &self.path
    }
}

/// Foundation of the API.
///
/// Walks the given paths, groups files by size (a side effect of the
/// metadata the walk already fetches, at no extra syscall cost), then only
/// opens files that share a size with at least one other file: a file with
/// a unique size can never be a duplicate, so it is never read.
pub fn find_dupes_partial<H, P>(
    directories: &[P],
    max_depth: Option<usize>,
    filter: filter::FileFilter,
    io_threads: usize,
) -> TreeBag<H::Hash, Candidate>
where
    H: crate::hasher::Hasher,
    P: AsRef<Path>,
{
    let by_size = collect_by_size(directories, max_depth, &filter);
    // Only files sharing a size get opened, so only those are worth warming.
    let queue = prefetch_queue(&by_size, |_| BLOCK_SIZE as u64);
    with_io_pool(io_threads, || {
        with_prefetch(queue, PARTIAL_PREFETCH_WINDOW, |progress| {
            partial_hash_by_size::<H>(by_size, progress)
        })
    })
}

/// Flattens the buckets that are actually going to be read into a queue for
/// the prefetcher, in iteration order. Singleton buckets are skipped: those
/// files are never opened.
fn prefetch_queue<K, V>(bag: &TreeBag<K, V>, len: impl Fn(&V) -> u64) -> Vec<(PathBuf, u64)>
where
    K: Ord,
    V: AsRef<Path>,
{
    bag.as_inner()
        .values()
        .filter(|bucket| bucket.len() > 1)
        .flat_map(|bucket| {
            bucket
                .iter()
                .map(|value| (value.as_ref().to_path_buf(), len(value)))
        })
        .collect()
}

/// Rehashes every bucket with more than one candidate to confirm (or rule
/// out) a real content match; buckets already known to be unique are
/// passed through untouched.
pub fn dedupe<H>(
    tree: TreeBag<H::Hash, Candidate>,
    io_threads: usize,
) -> crate::FileCounter<H::Hash>
where
    H: crate::hasher::Hasher,
{
    let queue = prefetch_queue(&tree, |candidate| candidate.size.min(CONTENT_PREFETCH_LEN));
    with_io_pool(io_threads, || {
        with_prefetch(queue, CONTENT_PREFETCH_WINDOW, |progress| {
            let (sender, receiver) = crossbeam_channel::bounded(CHANNEL_SIZE);
            rayon::join(
                move || receiver.into_iter().collect(),
                move || {
                    tree.into_inner().into_par_iter().for_each_with(
                        sender,
                        |sender, bucket: (H::Hash, Vec<Candidate>)| {
                            let read = bucket.1.len();
                            process_bucket::<H>(sender, bucket);
                            progress.fetch_add(read, Ordering::Relaxed);
                        },
                    )
                },
            )
            .0
        })
    })
}

/// Runs `work` while dedicated threads walk `queue` in order, asking the
/// kernel to start fetching each file before a hashing thread gets to it.
///
/// The prefetcher is kept on a leash: it never runs more than `window`
/// entries ahead of the progress counter that `work` is expected to advance
/// as it consumes files. Without that bound it would race to the end of the
/// queue and fill the page cache with data that gets evicted before anyone
/// reads it.
///
/// This can only ever affect timing. The hashing path is untouched and still
/// opens and reads every file itself, so a prefetch that is skipped, fails,
/// or lands too late costs speed and nothing else.
fn with_prefetch<T>(
    queue: Vec<(PathBuf, u64)>,
    window: usize,
    work: impl FnOnce(&AtomicUsize) -> T,
) -> T {
    let progress = AtomicUsize::new(0);
    if !advise::PREFETCH_SUPPORTED || queue.is_empty() {
        return work(&progress);
    }
    let cursor = AtomicUsize::new(0);
    let finished = AtomicBool::new(false);
    std::thread::scope(|scope| {
        for _ in 0..PREFETCH_THREADS.min(queue.len()) {
            scope.spawn(|| {
                loop {
                    let index = cursor.fetch_add(1, Ordering::Relaxed);
                    let Some((path, len)) = queue.get(index) else {
                        return;
                    };
                    // Wait for the hashers to catch up before running further
                    // ahead, and bail out entirely once they are done.
                    while index > progress.load(Ordering::Relaxed) + window {
                        if finished.load(Ordering::Acquire) {
                            return;
                        }
                        std::thread::sleep(std::time::Duration::from_micros(200));
                    }
                    if finished.load(Ordering::Acquire) {
                        return;
                    }
                    advise::prefetch(path, *len);
                }
            });
        }
        let result = work(&progress);
        finished.store(true, Ordering::Release);
        result
    })
}

fn with_io_pool<T: Send>(io_threads: usize, work: impl FnOnce() -> T + Send) -> T {
    static RAISE_NOFILE_LIMIT: std::sync::Once = std::sync::Once::new();
    RAISE_NOFILE_LIMIT.call_once(advise::raise_nofile_limit);
    let io_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(io_threads)
        .build()
        .expect("failed to build I/O thread pool");
    io_pool.install(work)
}

/// Walks `directories` and groups every matching file by its size.
fn collect_by_size<P>(
    directories: &[P],
    max_depth: Option<usize>,
    filter: &filter::FileFilter,
) -> TreeBag<u64, PathBuf>
where
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
        .threads(num_cpus::get())
        .build_parallel();
    let (sender, receiver) = crossbeam_channel::bounded(CHANNEL_SIZE);
    rayon::join(
        move || receiver.into_iter().collect(),
        move || {
            walker.for_each(|entry| {
                if let Err(error) = entry {
                    log::error!("{}", error);
                    return ignore::WalkState::Continue;
                }
                if let Some(key_value) = size_entry(filter, entry.unwrap()) {
                    if let Err(error) = sender.send(key_value) {
                        log::error!("{}, couldn't send value across channel", error);
                    }
                }
                ignore::WalkState::Continue
            })
        },
    )
    .0
}

fn size_entry(filter: &filter::FileFilter, entry: ignore::DirEntry) -> Option<(u64, PathBuf)> {
    let path = entry.path();
    let meta = entry
        .metadata()
        .map_err(|error| log::error!("{}, couldn't get metadata for {:?}", error, path))
        .ok()?;
    let len = meta.len();
    if !filter.is_match(path, meta) {
        return None;
    }
    Some((len, entry.into_path()))
}

/// Turns size-buckets into partial-hash buckets. Files that are the only
/// one of their size are never opened; the rest are read for their first
/// 4 KiB.
fn partial_hash_by_size<H>(
    by_size: TreeBag<u64, PathBuf>,
    progress: &AtomicUsize,
) -> TreeBag<H::Hash, Candidate>
where
    H: crate::hasher::Hasher,
{
    let (sender, receiver) = crossbeam_channel::bounded(CHANNEL_SIZE);
    rayon::join(
        move || receiver.into_iter().collect(),
        move || {
            by_size.into_inner().into_par_iter().for_each_with(
                sender,
                |sender, bucket: (u64, Vec<PathBuf>)| {
                    let read = bucket.1.len();
                    hash_size_bucket::<H>(sender, bucket);
                    progress.fetch_add(read, Ordering::Relaxed);
                },
            )
        },
    )
    .0
}

fn hash_size_bucket<H>(
    sender: &mut crossbeam_channel::Sender<(H::Hash, Candidate)>,
    (size, bucket): (u64, Vec<PathBuf>),
) where
    H: crate::hasher::Hasher,
{
    if bucket.len() == 1 {
        let path = bucket.into_iter().next().unwrap();
        let hash = hash::size_only::<H>(size);
        send(sender, hash, Candidate { path, size });
        return;
    }
    bucket
        .into_par_iter()
        .for_each_with(sender.clone(), |sender, path| {
            match hash::partial::<H>(&path, size) {
                Ok(hash) => send(sender, hash, Candidate { path, size }),
                Err(error) => log::error!("{}, couldn't hash {:?}", error, path),
            }
        });
}

fn process_bucket<H>(
    sender: &mut crossbeam_channel::Sender<(H::Hash, crate::Path)>,
    (old_hash, bucket): (H::Hash, Vec<Candidate>),
) where
    H: crate::hasher::Hasher,
{
    if bucket.len() == 1 {
        let candidate = bucket.into_iter().next().unwrap();
        send(sender, old_hash, candidate.path.into());
        return;
    }
    let (large, rest): (Vec<_>, Vec<_>) = bucket
        .into_iter()
        .partition(|candidate| candidate.size >= SUFFIX_HASH_THRESHOLD);

    rest.into_par_iter()
        .for_each_with(sender.clone(), |sender, candidate| {
            let hash = full_hash::<H>(&candidate).unwrap_or(old_hash);
            send(sender, hash, candidate.path.into());
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
        .map(|candidate| {
            let hash = suffix_hash::<H>(&candidate).unwrap_or(old_hash);
            (hash, candidate)
        })
        .collect::<Vec<_>>()
        .into_iter()
        .collect();
    by_suffix.into_inner().into_par_iter().for_each_with(
        sender.clone(),
        |sender, (suffix_hash, group)| {
            if group.len() == 1 {
                let candidate = group.into_iter().next().unwrap();
                send(sender, suffix_hash, candidate.path.into());
                return;
            }
            group
                .into_par_iter()
                .for_each_with(sender.clone(), |sender, candidate| {
                    let hash = full_hash::<H>(&candidate).unwrap_or(suffix_hash);
                    send(sender, hash, candidate.path.into());
                });
        },
    );
}

fn full_hash<H>(candidate: &Candidate) -> Result<H::Hash, ()>
where
    H: crate::hasher::Hasher,
{
    if candidate.size < BLOCK_SIZE as u64 {
        // Its partial hash already covered the whole content plus the
        // size: nothing more to distinguish it by.
        return Err(());
    }
    hash::full::<H>(&candidate.path).map_err(|error| {
        log::error!(
            "{}, couldn't hash {:?}, reusing previous hash",
            error,
            candidate.path
        )
    })
}

fn suffix_hash<H>(candidate: &Candidate) -> Result<H::Hash, ()>
where
    H: crate::hasher::Hasher,
{
    hash::suffix::<H>(&candidate.path, candidate.size).map_err(|error| {
        log::error!(
            "{}, couldn't hash suffix of {:?}, reusing previous hash",
            error,
            candidate.path
        )
    })
}

fn send<H, V>(sender: &mut crossbeam_channel::Sender<(H, V)>, hash: H, value: V) {
    if let Err(error) = sender.send((hash, value)) {
        log::error!("{}, couldn't send value across channel", error);
    }
}
