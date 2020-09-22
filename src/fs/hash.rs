//! Functions used to checksum files.
//!
//! # Examples :
//!
//! ```no_run
//! use yadf::fs::hash::FsHasher;
//! let hasher: FsHasher<seahash::SeaHasher> = Default::default();
//! let cargo_toml_hash = hasher.partial("./Cargo.toml").unwrap();
//! ```

use super::BLOCK_SIZE;
use std::fs::File;
use std::hash::Hasher;
use std::io::{self, Read};
use std::path::Path;

#[derive(Copy, Clone)]
pub struct FsHasher<H: Hasher + Default>(std::marker::PhantomData<H>);

impl<H> FsHasher<H>
where
    H: Hasher + Default,
{
    /// Get a checksum of the first 4 KiB (at most) of a file.
    pub fn partial<P: AsRef<Path>>(self, path: P) -> io::Result<u64> {
        let mut file = File::open(path)?;
        let mut buffer = [0u8; BLOCK_SIZE];
        let mut n = 0;
        loop {
            match file.read(&mut buffer[n..]) {
                Ok(0) => break,
                Ok(len) => n += len,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        let mut hasher = H::default();
        hasher.write(&buffer[..n]);
        Ok(hasher.finish())
    }

    /// Get a complete checksum of a file.
    pub fn full<P: AsRef<Path>>(self, path: P) -> io::Result<u64> {
        let mut file = File::open(path)?;
        let mut hasher = H::default();
        let mut buffer = [0u8; BLOCK_SIZE * 4];
        loop {
            match file.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => hasher.write(&buffer[..n]),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(hasher.finish())
    }
}

impl<H: Hasher + Default> Default for FsHasher<H> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use twox_hash::XxHash64;

    #[test]
    fn same_hash_partial_and_full_for_small_file() {
        let hasher: FsHasher<XxHash64> = FsHasher::default();
        let h1 = hasher.partial("./tests/static/foo").unwrap();
        let h2 = hasher.full("./tests/static/foo").unwrap();
        assert_eq!(h1, h2);
    }
}
