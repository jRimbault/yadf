//! Keeping the device queue deep: warming files into the page cache just
//! before the hashing passes reach them.
//!
//! A [`Queue`] is the list of files a pass is going to open, in the order it
//! will open them. [`Queue::warm`] runs that pass with dedicated threads
//! walking the queue ahead of it, asking the kernel to start each read
//! early. The pass reports how far it has got through a [`Progress`] handle,
//! which is what keeps the prefetcher on a leash.
//!
//! This can only ever affect timing. The hashing path is untouched and still
//! opens and reads every file itself, so a prefetch that is skipped, fails,
//! or lands too late costs speed and nothing else.

use super::advise;
use crate::units::Bytes;
use crate::TreeBag;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

/// Threads dedicated to issuing readahead hints. They only ever `open` and
/// `posix_fadvise`, never read or hash, so they cost almost no CPU and can
/// safely outnumber the cores.
///
/// All four prefetch constants in this module were swept on a 150k-file /
/// 27.6 GB corpus. Cold and warm wall time turned out to be insensitive to
/// every one of them: threads 8-128, partial window 256-16384, content
/// window 32-2048 and content length 4 KiB-1 MiB all landed within ~1% of
/// each other, which is the run-to-run noise. They are therefore chosen for
/// the secondary criteria -- syscall cost and page-cache pressure -- rather
/// than tuned to a sharp optimum. Do not treat them as finely calibrated.
const THREADS: usize = 16;

/// How long a prefetch thread waits before re-checking whether the hashers
/// have moved far enough for it to run on.
const BACKOFF: std::time::Duration = std::time::Duration::from_micros(200);

/// How much of a file to warm ahead of the content-hashing pass.
///
/// Deliberately small: the gain comes from getting a file's read *started*
/// before a hashing thread blocks on it, not from bulk-loading it. A 4 KiB
/// hint measured exactly as fast as a 1 MiB one while spending ~10% less
/// system time, and once the read is under way `POSIX_FADV_SEQUENTIAL` keeps
/// it fed anyway.
pub const CONTENT_HEAD: Bytes = Bytes::kib(16);

/// How far ahead of the hashing threads the prefetcher may run, in files.
///
/// Bounds how much speculative data can sit in the page cache. The leash
/// does earn its keep: letting the prefetcher run unbounded (a window past
/// the file count) was the one setting that measured consistently worse.
#[derive(Debug, Clone, Copy)]
pub struct Window(usize);

impl Window {
    /// For the partial pass, which reads a single block per file and so
    /// gets through the queue quickly.
    pub const PARTIAL: Self = Self(4096);
    /// For the content pass, where a file in the window may be read whole.
    pub const CONTENT: Self = Self(64);
}

/// A file to warm, and how much of it.
#[derive(Debug)]
struct Request {
    path: PathBuf,
    len: Bytes,
}

/// The files a hashing pass is going to open, in the order it will open
/// them.
#[derive(Debug)]
pub struct Queue(Vec<Request>);

/// How many files the hashing pass has consumed so far. Shared with the
/// prefetch threads, which use it to stay a bounded distance ahead.
#[derive(Debug, Default)]
pub struct Progress(AtomicUsize);

impl Progress {
    /// Reports `files` more files consumed. Called once per bucket rather
    /// than once per file, so the prefetcher's view lags slightly -- which
    /// only ever makes it more conservative.
    pub fn advance(&self, files: usize) {
        self.0.fetch_add(files, Ordering::Relaxed);
    }

    fn consumed(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }
}

impl Queue {
    /// Flattens the buckets of `bag` that are actually going to be read into
    /// a queue, in iteration order; `len` says how much of each file to
    /// warm. Singleton buckets are skipped: those files are never opened.
    pub fn covering<K, V>(bag: &TreeBag<K, V>, len: impl Fn(&V) -> Bytes) -> Self
    where
        K: Ord,
        V: AsRef<Path>,
    {
        Self(
            bag.as_inner()
                .values()
                .filter(|bucket| bucket.len() > 1)
                .flat_map(|bucket| {
                    bucket.iter().map(|value| Request {
                        path: value.as_ref().to_path_buf(),
                        len: len(value),
                    })
                })
                .collect(),
        )
    }

    /// Runs `work` while dedicated threads warm this queue ahead of it.
    ///
    /// `work` is handed the [`Progress`] handle it is expected to advance as
    /// it consumes files; the prefetcher never runs more than `window`
    /// entries past it. Without that bound it would race to the end of the
    /// queue and fill the page cache with data that gets evicted before
    /// anyone reads it.
    pub fn warm<T>(&self, window: Window, work: impl FnOnce(&Progress) -> T) -> T {
        let progress = Progress::default();
        if !advise::PREFETCH_SUPPORTED || self.0.is_empty() {
            return work(&progress);
        }
        let cursor = AtomicUsize::new(0);
        let finished = AtomicBool::new(false);
        std::thread::scope(|scope| {
            for _ in 0..THREADS.min(self.0.len()) {
                scope.spawn(|| self.warm_ahead_of(&progress, window, &cursor, &finished));
            }
            let result = work(&progress);
            finished.store(true, Ordering::Release);
            result
        })
    }

    /// One prefetch thread: claim the next request, wait for the hashers to
    /// come within `window` of it, warm it, repeat until the queue is
    /// exhausted or `work` is done.
    fn warm_ahead_of(
        &self,
        progress: &Progress,
        window: Window,
        cursor: &AtomicUsize,
        finished: &AtomicBool,
    ) {
        loop {
            let index = cursor.fetch_add(1, Ordering::Relaxed);
            let Some(request) = self.0.get(index) else {
                return;
            };
            while index > progress.consumed() + window.0 {
                if finished.load(Ordering::Acquire) {
                    return;
                }
                std::thread::sleep(BACKOFF);
            }
            if finished.load(Ordering::Acquire) {
                return;
            }
            advise::prefetch(&request.path, request.len);
        }
    }
}
