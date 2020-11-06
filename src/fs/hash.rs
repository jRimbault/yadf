use super::BLOCK_SIZE;
use std::fs::File;
use std::hash::Hasher;
use std::io::{self, Read};
use std::path::Path;

#[derive(Copy, Clone)]
pub struct FsHasher<H>(std::marker::PhantomData<H>)
where
    H: Hasher + Default,
    H: std::io::Write;

impl<H> FsHasher<H>
where
    H: Hasher + Default,
    H: std::io::Write,
{
    /// Get a checksum of the first 4 KiB (at most) of a file.
    pub fn partial<P: AsRef<Path>>(self, path: &P) -> io::Result<u64> {
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
        Hasher::write(&mut hasher, &buffer[..n]);
        Ok(hasher.finish())
    }

    /// Get a complete checksum of a file.
    pub fn full<P: AsRef<Path>>(self, path: &P) -> io::Result<u64> {
        let mut file = File::open(path)?;
        let mut hasher = H::default();
        io::copy(&mut file, &mut hasher)?;
        Ok(hasher.finish())
    }
}

impl<H> Default for FsHasher<H>
where
    H: Hasher + Default,
    H: std::io::Write,
{
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_hash_partial_and_full_for_small_file() {
        let hasher: FsHasher<crate::XxHasher> = FsHasher::default();
        let h1 = hasher.partial(&"./tests/static/foo").unwrap();
        let hasher: FsHasher<crate::XxHasher> = FsHasher::default();
        let h2 = hasher.full(&"./tests/static/foo").unwrap();
        assert_eq!(h1, h2);
    }
}
