use super::BLOCK_SIZE;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Get a checksum of the first 4 KiB (at most) of a file.
pub fn partial<H>(path: &Path) -> io::Result<H::Hash>
where
    H: crate::hasher::Hasher,
{
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
    hasher.write(&file.metadata()?.len().to_le_bytes());
    hasher.write(&buffer[..n]);
    Ok(hasher.finish())
}

/// Get a complete checksum of a file.
pub fn full<H>(path: &Path) -> io::Result<H::Hash>
where
    H: crate::hasher::Hasher,
{
    /// Compile time [`Write`](std::io::Write) wrapper for a [`Hasher`](core::hash::Hasher).
    /// This should get erased at compile time.
    #[repr(transparent)]
    struct HashWriter<H>(H);

    impl<H: crate::hasher::Hasher> io::Write for HashWriter<H> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            crate::hasher::Hasher::write(&mut self.0, buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    let mut hasher = HashWriter(H::default());
    io::copy(&mut File::open(path)?, &mut hasher)?;
    Ok(hasher.0.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_hash_partial_and_full_for_small_file_because_of_size() {
        let h1 = partial::<seahash::SeaHasher>("./tests/static/foo".as_ref()).unwrap();
        let h2 = full::<seahash::SeaHasher>("./tests/static/foo".as_ref()).unwrap();
        assert_ne!(h1, h2);
    }
}
