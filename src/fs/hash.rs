use super::BLOCK_SIZE;
use std::fs::File;
use std::hash::Hasher;
use std::io::{self, Read};
use std::path::Path;

/// Get a checksum of the first 4 KiB (at most) of a file.
pub fn partial<H1, H2, P>(path: &P) -> io::Result<(u64, u64)>
where
    H1: Hasher + Default,
    H2: Hasher + Default,
    P: AsRef<Path>,
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
    let mut hasher1 = H1::default();
    hasher1.write(&buffer[..n]);
    let mut hasher2 = H2::default();
    hasher2.write(&buffer[..n]);
    Ok((hasher1.finish(), hasher2.finish()))
}

/// Get a complete checksum of a file.
pub fn full<H1, H2, P>(path: &P) -> io::Result<(u64, u64)>
where
    H1: Hasher + Default,
    H2: Hasher + Default,
    P: AsRef<Path>,
{
    /// Compile time [`Write`](std::io::Write) wrapper for a [`Hasher`](core::hash::Hasher).
    /// This should get erased at compile time.
    struct DoubleHashWriter<H1, H2>(H1, H2);

    impl<H1: Hasher, H2: Hasher> io::Write for DoubleHashWriter<H1, H2> {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.write(buf);
            self.1.write(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let mut hasher = DoubleHashWriter(H1::default(), H2::default());
    io::copy(&mut File::open(path)?, &mut hasher)?;
    Ok((hasher.0.finish(), hasher.1.finish()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_hash_partial_and_full_for_small_file() {
        use seahash::SeaHasher;
        let h1 = partial::<SeaHasher, SeaHasher, _>(&"./tests/static/foo").unwrap();
        let h2 = full::<SeaHasher, SeaHasher, _>(&"./tests/static/foo").unwrap();
        assert_eq!(h1, h2);
    }
}
