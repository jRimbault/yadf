//! A read-only file handle that speaks in intents rather than syscalls.
//!
//! Callers say how they mean to read a file ([`Access`]) and what they want
//! out of it (a prefix, a slice at an offset, the whole content); the
//! platform tricks -- `O_NOATIME`, `posix_fadvise`, positional reads -- and
//! the short-read/`EINTR` retry loops stay behind this boundary.

use super::advise;
use crate::units::Bytes;
use std::io::{self, Read};
use std::path::Path;

/// Buffer size for the full-file streaming pass. Large enough that a big
/// file is read in a handful of syscalls rather than hundreds.
const SCRATCH_SIZE: usize = 256 * 1024;

thread_local! {
    // Reused across calls on the same I/O-pool thread so hashing many
    // large files in a row doesn't repeatedly allocate/free 256 KiB.
    static SCRATCH: std::cell::RefCell<Vec<u8>> = std::cell::RefCell::new(vec![0u8; SCRATCH_SIZE]);
}

/// How the caller intends to read a file. Passed to the kernel as a hint so
/// it can size its readahead accordingly.
#[derive(Debug, Clone, Copy)]
pub enum Access {
    /// A short read of one region and nothing else: readahead past it would
    /// just waste bandwidth on a cold cache.
    Random,
    /// A front-to-back read of the whole file, which does want readahead.
    Sequential,
}

/// A file opened for reading, with atime updates suppressed where the
/// platform allows it.
#[derive(Debug)]
pub struct Reader(std::fs::File);

impl Reader {
    pub fn open(path: &Path, access: Access) -> io::Result<Self> {
        let file = advise::open_noatime(path)?;
        advise::advise(&file, access);
        Ok(Self(file))
    }

    /// Reads up to `buffer.len()` bytes from the start of the file and
    /// returns the slice actually filled, which is shorter only if the file
    /// itself is. Short and interrupted reads are retried.
    pub fn read_prefix<'b>(&mut self, buffer: &'b mut [u8]) -> io::Result<&'b [u8]> {
        let mut filled = 0;
        while filled < buffer.len() {
            match self.0.read(&mut buffer[filled..]) {
                Ok(0) => break,
                Ok(read) => filled += read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        }
        Ok(&buffer[..filled])
    }

    /// Fills `buffer` from `offset`, in a single positional read rather than
    /// a `seek` + `read` pair where the platform allows it.
    pub fn read_exact_at(&self, buffer: &mut [u8], offset: Bytes) -> io::Result<()> {
        read_exact_at(&self.0, buffer, offset.get())
    }

    /// Streams the whole file through `sink`, chunk by chunk, using a
    /// buffer shared by every call on this thread.
    ///
    /// `sink` must therefore not itself read a file through this method:
    /// the shared buffer is not re-entrant.
    pub fn for_each_chunk(&mut self, mut sink: impl FnMut(&[u8])) -> io::Result<()> {
        SCRATCH.with_borrow_mut(|buffer| loop {
            match self.0.read(buffer) {
                Ok(0) => return Ok(()),
                Ok(read) => sink(&buffer[..read]),
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(error) => return Err(error),
            }
        })
    }
}

#[cfg(unix)]
fn read_exact_at(file: &std::fs::File, buffer: &mut [u8], offset: u64) -> io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.read_exact_at(buffer, offset)
}

#[cfg(not(unix))]
fn read_exact_at(file: &std::fs::File, buffer: &mut [u8], offset: u64) -> io::Result<()> {
    use std::io::{Seek, SeekFrom};
    let mut file = file;
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(buffer)
}
