//! Platform layer: best-effort `posix_fadvise` hints, `O_NOATIME` opens and
//! rlimit raising. Everything here is a hint or a perk -- failures
//! (unsupported filesystem, non-Linux target) are silently ignored and must
//! never affect correctness.
//!
//! Typed access to this module goes through [`super::file::Reader`] and
//! [`super::prefetch`]; nothing else should reach for it directly.

use super::file::Access;
use crate::units::Bytes;
use std::fs::File;
use std::io;
use std::path::Path;

/// Tells the kernel how `file` is about to be read.
///
/// `Random` stops it wasting bandwidth fetching bytes past a 4 KiB prefix
/// read on a cold cache; `Sequential` widens readahead for the full-file
/// pass, which does want it.
#[cfg(target_os = "linux")]
pub fn advise(file: &File, access: Access) {
    let advice = match access {
        Access::Random => rustix::fs::Advice::Random,
        Access::Sequential => rustix::fs::Advice::Sequential,
    };
    let _ = rustix::fs::fadvise(file, 0, None, advice);
}

#[cfg(not(target_os = "linux"))]
pub fn advise(_file: &File, _access: Access) {}

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

/// Whether [`prefetch`] does anything on this platform. Callers use this to
/// skip spawning prefetch threads that would have nothing to do.
pub const PREFETCH_SUPPORTED: bool = cfg!(target_os = "linux");

/// Asks the kernel to start pulling the first `len` bytes of `path` into the
/// page cache, without reading them.
///
/// This is the whole point of the prefetcher: `POSIX_FADV_WILLNEED` queues an
/// asynchronous read at the device and returns immediately, so a thread that
/// issues it does not block waiting for the data. That raises the number of
/// requests in flight without adding threads that also *hash* — which is what
/// makes it different from simply raising `--io-threads`, where every extra
/// thread contends for CPU on a warm cache.
///
/// Purely an optimisation: it warms the cache and nothing else, so a failure
/// anywhere here can only cost speed, never correctness.
#[cfg(target_os = "linux")]
pub fn prefetch(path: &Path, len: Bytes) {
    let Some(len) = std::num::NonZeroU64::new(len.get()) else {
        return;
    };
    if let Ok(file) = open_noatime(path) {
        let _ = rustix::fs::fadvise(&file, 0, Some(len), rustix::fs::Advice::WillNeed);
    }
}

#[cfg(not(target_os = "linux"))]
pub fn prefetch(_path: &Path, _len: Bytes) {}

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
