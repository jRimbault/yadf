use super::advise::{self, Advice};
use super::BLOCK_SIZE;
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

/// Buffer size for the full-file hashing pass. Large enough that a big
/// file is read in a handful of syscalls rather than hundreds.
const FULL_READ_BUFFER_SIZE: usize = 256 * 1024;

thread_local! {
    // Reused across calls on the same I/O-pool thread so hashing many
    // large files in a row doesn't repeatedly allocate/free 256 KiB.
    static FULL_READ_BUFFER: std::cell::RefCell<Vec<u8>> =
        std::cell::RefCell::new(vec![0u8; FULL_READ_BUFFER_SIZE]);
}

/// Get a checksum for a file known (from a prior size-grouping pass) to be
/// the only file of its size, and therefore guaranteed unique. Never opens
/// the file.
pub fn size_only<H>(len: u64) -> H::Hash
where
    H: crate::hasher::Hasher,
{
    let mut hasher = H::default();
    hasher.write(&len.to_le_bytes());
    hasher.finish()
}

/// Get a checksum of the first 4 KiB (at most) of a file.
///
/// `len` is the already-known file size (from the caller's earlier
/// `stat`), so this never issues its own `fstat`.
pub fn partial<H>(path: &Path, len: u64) -> io::Result<H::Hash>
where
    H: crate::hasher::Hasher,
{
    let mut file = advise::open_noatime(path)?;
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

/// Get a checksum of the last 4 KiB (at most) of a file, in a single
/// positional read rather than a `seek` + `read` pair where the platform
/// allows it. Cheap way to split apart large files that only share a
/// header before paying for a full read.
pub fn suffix<H>(path: &Path, len: u64) -> io::Result<H::Hash>
where
    H: crate::hasher::Hasher,
{
    let file = advise::open_noatime(path)?;
    advise::advise(&file, Advice::Random);
    let read_len = (len as usize).min(BLOCK_SIZE);
    let offset = len - read_len as u64;
    let mut buffer = [0u8; BLOCK_SIZE];
    read_at(&file, &mut buffer[..read_len], offset)?;
    let mut hasher = H::default();
    hasher.write(&buffer[..read_len]);
    Ok(hasher.finish())
}

#[cfg(unix)]
fn read_at(file: &File, buffer: &mut [u8], offset: u64) -> io::Result<()> {
    use std::os::unix::fs::FileExt;
    file.read_exact_at(buffer, offset)
}

#[cfg(not(unix))]
fn read_at(file: &File, buffer: &mut [u8], offset: u64) -> io::Result<()> {
    use std::io::{Seek, SeekFrom};
    let mut file = file;
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(buffer)
}

/// Get a complete checksum of a file.
pub fn full<H>(path: &Path) -> io::Result<H::Hash>
where
    H: crate::hasher::Hasher,
{
    let mut file = advise::open_noatime(path)?;
    advise::advise(&file, Advice::Sequential);
    let mut hasher = H::default();
    FULL_READ_BUFFER.with_borrow_mut(|buffer| -> io::Result<()> {
        loop {
            match file.read(buffer) {
                Ok(0) => break,
                Ok(n) => hasher.write(&buffer[..n]),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(())
    })?;
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

    #[test]
    fn suffix_hash_reads_the_tail_not_the_head() {
        let dir = tempdir();
        let path = dir.join("suffix-test");
        std::fs::write(&path, [b'a'; 8192]).unwrap();
        let h_all_a = suffix::<seahash::SeaHasher>(&path, 8192).unwrap();
        let mut content = vec![b'a'; 8192];
        content[8191] = b'b';
        std::fs::write(&path, &content).unwrap();
        let h_last_byte_differs = suffix::<seahash::SeaHasher>(&path, 8192).unwrap();
        assert_ne!(h_all_a, h_last_byte_differs);

        let mut content = vec![b'a'; 8192];
        content[0] = b'b';
        std::fs::write(&path, &content).unwrap();
        let h_first_byte_differs = suffix::<seahash::SeaHasher>(&path, 8192).unwrap();
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
