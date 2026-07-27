//! Checksumming a file the cheapest way that can still tell it apart from
//! the files it shares a size with: its size alone, a 4 KiB prefix, a 4 KiB
//! suffix, or its whole content.

use super::file::{Access, Reader};
use crate::units::Bytes;
use std::io;
use std::path::Path;

/// How much of a file the partial passes look at, and the unit the
/// prefetcher warms ahead of them.
pub const BLOCK: Bytes = Bytes::kib(4);
const BLOCK_LEN: usize = BLOCK.get() as usize;

/// Get a checksum for a file known (from a prior size-grouping pass) to be
/// the only file of its size, and therefore guaranteed unique. Never opens
/// the file.
pub fn size_only<H>(size: Bytes) -> H::Hash
where
    H: crate::hasher::Hasher,
{
    let mut hasher = H::default();
    hasher.write(&size.to_le_bytes());
    hasher.finish()
}

/// Get a checksum of the first 4 KiB (at most) of a file.
///
/// `size` is the already-known file size (from the caller's earlier
/// `stat`), so this never issues its own `fstat`.
pub fn partial<H>(path: &Path, size: Bytes) -> io::Result<H::Hash>
where
    H: crate::hasher::Hasher,
{
    let mut file = Reader::open(path, Access::Random)?;
    let mut buffer = [0u8; BLOCK_LEN];
    let prefix = file.read_prefix(&mut buffer)?;
    let mut hasher = H::default();
    hasher.write(&size.to_le_bytes());
    hasher.write(prefix);
    Ok(hasher.finish())
}

/// Get a checksum of the last 4 KiB (at most) of a file. Cheap way to split
/// apart large files that only share a header before paying for a full read.
pub fn suffix<H>(path: &Path, size: Bytes) -> io::Result<H::Hash>
where
    H: crate::hasher::Hasher,
{
    let file = Reader::open(path, Access::Random)?;
    let len = size.min(BLOCK);
    let mut buffer = [0u8; BLOCK_LEN];
    let tail = &mut buffer[..len.as_usize()];
    file.read_exact_at(tail, size - len)?;
    let mut hasher = H::default();
    hasher.write(tail);
    Ok(hasher.finish())
}

/// Get a complete checksum of a file.
pub fn full<H>(path: &Path) -> io::Result<H::Hash>
where
    H: crate::hasher::Hasher,
{
    let mut file = Reader::open(path, Access::Sequential)?;
    let mut hasher = H::default();
    file.for_each_chunk(|chunk| hasher.write(chunk))?;
    Ok(hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn different_hash_partial_and_full_for_small_file_because_of_size() {
        let path: &Path = "./tests/static/foo".as_ref();
        let size = Bytes::new(std::fs::metadata(path).unwrap().len());
        let h1 = partial::<seahash::SeaHasher>(path, size).unwrap();
        let h2 = full::<seahash::SeaHasher>(path).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn suffix_hash_reads_the_tail_not_the_head() {
        let dir = tempdir();
        let path = dir.join("suffix-test");
        let size = Bytes::new(8192);
        std::fs::write(&path, [b'a'; 8192]).unwrap();
        let h_all_a = suffix::<seahash::SeaHasher>(&path, size).unwrap();
        let mut content = vec![b'a'; 8192];
        content[8191] = b'b';
        std::fs::write(&path, &content).unwrap();
        let h_last_byte_differs = suffix::<seahash::SeaHasher>(&path, size).unwrap();
        assert_ne!(h_all_a, h_last_byte_differs);

        let mut content = vec![b'a'; 8192];
        content[0] = b'b';
        std::fs::write(&path, &content).unwrap();
        let h_first_byte_differs = suffix::<seahash::SeaHasher>(&path, size).unwrap();
        assert_eq!(
            h_all_a, h_first_byte_differs,
            "suffix hash must not be affected by a change outside the last 4 KiB"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("yadf-hash-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
