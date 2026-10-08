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

/// Which checksum a hash is, fed to the hasher ahead of everything else.
///
/// The hashes of every pass end up as keys of the same bag. Each one is
/// over the pass, the file size, then the content the pass reads, whose
/// length the first two fix: two passes never hash the same input, so,
/// short of a digest collision, the prefix of one file can't pass for the
/// whole of another that happens to start with the first's size.
///
/// That doesn't make a sampled hash a file's identity: different files do
/// share a prefix or a suffix. The pipeline only reports a file under one
/// once no other file of its size can share it.
#[derive(Clone, Copy)]
#[repr(u8)]
enum Pass {
    Size,
    Prefix,
    Suffix,
    Full,
}

fn hasher<H>(pass: Pass, size: Bytes) -> H
where
    H: crate::hasher::Hasher,
{
    let mut hasher = H::default();
    hasher.write(&[pass as u8]);
    hasher.write(&size.to_le_bytes());
    hasher
}

/// Get a checksum for a file known (from a prior size-grouping pass) to be
/// the only file of its size, and therefore guaranteed unique. Never opens
/// the file.
pub fn size_only<H>(size: Bytes) -> H::Hash
where
    H: crate::hasher::Hasher,
{
    hasher::<H>(Pass::Size, size).finish()
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
    let mut hasher = hasher::<H>(Pass::Prefix, size);
    hasher.write(prefix);
    Ok(hasher.finish())
}

/// Get a checksum of the first and last 4 KiB of a file, at least 8 KiB
/// long. Cheap way to split apart large files that only share a header
/// before paying for a full read.
///
/// Each bucket of a shared prefix is split on its own, and a file left
/// alone is reported under this hash, so it covers the prefix again: two
/// lone files of different buckets that share a tail must not share it.
pub fn suffix<H>(path: &Path, size: Bytes) -> io::Result<H::Hash>
where
    H: crate::hasher::Hasher,
{
    debug_assert!(size >= Bytes::kib(8), "head and tail would overlap");
    let file = Reader::open(path, Access::Random)?;
    let mut buffer = [0u8; BLOCK_LEN];
    let mut hasher = hasher::<H>(Pass::Suffix, size);
    file.read_exact_at(&mut buffer, Bytes::new(0))?;
    hasher.write(&buffer);
    file.read_exact_at(&mut buffer, size - BLOCK)?;
    hasher.write(&buffer);
    Ok(hasher.finish())
}

/// Get a complete checksum of a file.
///
/// `size` is the already-known file size, hashed ahead of the content.
pub fn full<H>(path: &Path, size: Bytes) -> io::Result<H::Hash>
where
    H: crate::hasher::Hasher,
{
    let mut file = Reader::open(path, Access::Sequential)?;
    let mut hasher = hasher::<H>(Pass::Full, size);
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
        let h2 = full::<seahash::SeaHasher>(path, size).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn suffix_hash_reads_the_head_and_the_tail_not_the_middle() {
        let dir = tempdir();
        let path = dir.join("suffix-test");
        let size = Bytes::new(12288);
        let hash_with_byte_changed = |at: Option<usize>| {
            let mut content = vec![b'a'; 12288];
            if let Some(at) = at {
                content[at] = b'b';
            }
            std::fs::write(&path, &content).unwrap();
            suffix::<seahash::SeaHasher>(&path, size).unwrap()
        };
        let h_all_a = hash_with_byte_changed(None);
        assert_ne!(h_all_a, hash_with_byte_changed(Some(12287)));
        assert_ne!(h_all_a, hash_with_byte_changed(Some(0)));
        assert_eq!(
            h_all_a,
            hash_with_byte_changed(Some(6000)),
            "suffix hash must not be affected by a change between the first and last 4 KiB"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn tempdir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("yadf-hash-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
