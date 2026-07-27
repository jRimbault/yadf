use super::advise::{self, Advice};
use super::BLOCK_SIZE;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Buffer size for the full-file hashing pass. Large enough that a big
/// file is read in a handful of syscalls rather than hundreds.
const FULL_READ_BUFFER_SIZE: usize = 256 * 1024;

/// Get a checksum of the first 4 KiB (at most) of a file.
///
/// `len` is the already-known file size (from the caller's earlier
/// `stat`), so this never issues its own `fstat`.
pub fn partial<H>(path: &Path, len: u64) -> io::Result<H::Hash>
where
    H: crate::hasher::Hasher,
{
    let mut file = File::open(path)?;
    advise::advise(&file, Advice::Random);
    let mut buffer = [0u8; BLOCK_SIZE];
    let mut n = 0;
    while n < BLOCK_SIZE {
        match file.read(&mut buffer[n..]) {
            Ok(0) => break,
            Ok(read) => n += read,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    let mut hasher = H::default();
    hasher.write(&len.to_le_bytes());
    hasher.write(&buffer[..n]);
    Ok(hasher.finish())
}

/// Get a complete checksum of a file.
pub fn full<H>(path: &Path) -> io::Result<H::Hash>
where
    H: crate::hasher::Hasher,
{
    let mut file = File::open(path)?;
    advise::advise(&file, Advice::Sequential);
    let mut hasher = H::default();
    let mut buffer = vec![0u8; FULL_READ_BUFFER_SIZE];
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_hash_partial_and_full_for_small_file_because_of_size() {
        let path: &Path = "./tests/static/foo".as_ref();
        let len = std::fs::metadata(path).unwrap().len();
        let h1 = partial::<seahash::SeaHasher>(path, len).unwrap();
        let h2 = full::<seahash::SeaHasher>(path).unwrap();
        assert_ne!(h1, h2);
    }
}
