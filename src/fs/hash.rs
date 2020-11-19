use super::BLOCK_SIZE;
use std::fs::File;
use std::hash::Hasher;
use std::io::{self, Read};
use std::path::Path;

/// Get a checksum of the first 4 KiB (at most) of a file.
pub fn partial<H, P>(path: &P) -> io::Result<u64>
where
    H: crate::Hasher,
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
    let mut hasher = H::default();
    Hasher::write(&mut hasher, &buffer[..n]);
    Ok(hasher.finish())
}

/// Get a complete checksum of a file.
pub fn full<H, P>(path: &P) -> io::Result<u64>
where
    H: crate::Hasher,
    P: AsRef<Path>,
{
    let mut hasher = H::default();
    io::copy(&mut File::open(path)?, &mut hasher)?;
    Ok(hasher.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_hash_partial_and_full_for_small_file() {
        let h1 = partial::<crate::SeaHasher, _>(&"./tests/static/foo").unwrap();
        let h2 = full::<crate::SeaHasher, _>(&"./tests/static/foo").unwrap();
        assert_eq!(h1, h2);
    }
}
