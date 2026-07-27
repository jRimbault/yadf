//! Best-effort `posix_fadvise` hints.
//!
//! `Random` tells the kernel not to bother with readahead, which otherwise
//! wastes bandwidth fetching bytes past a 4 KiB prefix read on a cold cache.
//! `Sequential` widens readahead for the full-file pass, which does want it.
//! These are hints: failures (unsupported filesystem, non-Linux target) are
//! silently ignored and must never affect correctness.

use std::fs::File;

pub enum Advice {
    Random,
    Sequential,
}

#[cfg(target_os = "linux")]
pub fn advise(file: &File, advice: Advice) {
    let advice = match advice {
        Advice::Random => rustix::fs::Advice::Random,
        Advice::Sequential => rustix::fs::Advice::Sequential,
    };
    let _ = rustix::fs::fadvise(file, 0, None, advice);
}

#[cfg(not(target_os = "linux"))]
pub fn advise(_file: &File, _advice: Advice) {}
