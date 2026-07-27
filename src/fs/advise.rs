//! Best-effort `posix_fadvise` hints and `O_NOATIME` opens.
//!
//! `Random` tells the kernel not to bother with readahead, which otherwise
//! wastes bandwidth fetching bytes past a 4 KiB prefix read on a cold cache.
//! `Sequential` widens readahead for the full-file pass, which does want it.
//! These are hints: failures (unsupported filesystem, non-Linux target) are
//! silently ignored and must never affect correctness.

use std::fs::File;
use std::io;
use std::path::Path;

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

/// Opens a file for reading, asking the kernel to skip the atime update
/// every read would otherwise trigger. Falls back to a plain open on
/// `EPERM` (e.g. a file owned by another user), since `O_NOATIME` is a
/// perk, not a correctness requirement.
#[cfg(target_os = "linux")]
pub fn open_noatime(path: &Path) -> io::Result<File> {
    use rustix::fs::{Mode, OFlags};
    let flags = OFlags::RDONLY | OFlags::NOATIME;
    match rustix::fs::open(path, flags, Mode::empty()) {
        Ok(fd) => Ok(File::from(fd)),
        Err(rustix::io::Errno::PERM) => File::open(path),
        Err(error) => Err(error.into()),
    }
}

#[cfg(not(target_os = "linux"))]
pub fn open_noatime(path: &Path) -> io::Result<File> {
    File::open(path)
}

/// Best-effort raise of the open-file soft limit to the hard limit.
///
/// `--io-threads` above the core count, plus the walker's own pool, can
/// open enough files concurrently to exceed a distro-default 1024 soft
/// `RLIMIT_NOFILE` on a many-core machine. Failure here is not fatal: the
/// process just keeps whatever limit it started with, and normal I/O
/// errors surface if that turns out to be too low.
#[cfg(target_os = "linux")]
pub fn raise_nofile_limit() {
    use rustix::process::{getrlimit, setrlimit, Resource};
    let limit = getrlimit(Resource::Nofile);
    if let Some(maximum) = limit.maximum {
        let raised = rustix::process::Rlimit {
            current: Some(maximum),
            maximum: Some(maximum),
        };
        let _ = setrlimit(Resource::Nofile, raised);
    }
}

#[cfg(not(target_os = "linux"))]
pub fn raise_nofile_limit() {}
