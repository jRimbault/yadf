pub trait Hasher: Default {
    type Hash: Hash;
    fn write(&mut self, buf: &[u8]);
    fn finish(self) -> Self::Hash;
}

pub trait Hash: PartialEq + Eq + PartialOrd + Ord + Send + Sync + Copy {}

impl<T> Hash for T where T: PartialEq + Eq + PartialOrd + Ord + Send + Sync + Copy {}

#[cfg(feature = "build-bin")]
impl Hasher for ahash::AHasher {
    type Hash = u64;
    fn write(&mut self, buf: &[u8]) {
        std::hash::Hasher::write(self, buf);
    }
    fn finish(self) -> Self::Hash {
        std::hash::Hasher::finish(&self)
    }
}

#[cfg(feature = "build-bin")]
impl Hasher for highway::HighwayHasher {
    type Hash = [u64; 4];
    fn write(&mut self, buf: &[u8]) {
        use highway::HighwayHash;
        self.append(buf);
    }

    fn finish(self) -> Self::Hash {
        use highway::HighwayHash;
        self.finalize256()
    }
}

#[cfg(feature = "build-bin")]
impl Hasher for metrohash::MetroHash128 {
    type Hash = (u64, u64);
    fn write(&mut self, buf: &[u8]) {
        std::hash::Hasher::write(self, buf);
    }

    fn finish(self) -> Self::Hash {
        self.finish128()
    }
}

#[cfg(feature = "build-bin")]
impl Hasher for seahash::SeaHasher {
    type Hash = u64;
    fn write(&mut self, buf: &[u8]) {
        std::hash::Hasher::write(self, buf);
    }
    fn finish(self) -> Self::Hash {
        std::hash::Hasher::finish(&self)
    }
}

#[cfg(feature = "build-bin")]
impl Hasher for twox_hash::xxhash3_128::Hasher {
    type Hash = u128;
    fn write(&mut self, buf: &[u8]) {
        self.write(buf);
    }

    fn finish(self) -> Self::Hash {
        self.finish_128()
    }
}

#[cfg(feature = "build-bin")]
impl Hasher for blake3::Hasher {
    type Hash = [u8; 32];
    fn write(&mut self, buf: &[u8]) {
        self.update(buf);
    }
    fn finish(self) -> Self::Hash {
        self.finalize().into()
    }
}

#[cfg(all(test, feature = "build-bin"))]
mod tests {
    use super::*;

    fn hashes_are_consistent<H: Hasher>()
    where
        H::Hash: std::fmt::Debug,
    {
        let mut a = H::default();
        a.write(b"hello world");
        let mut b = H::default();
        b.write(b"hello world");
        assert_eq!(a.finish(), b.finish());
    }

    fn hashes_differ_on_different_input<H: Hasher>()
    where
        H::Hash: std::fmt::Debug,
    {
        let mut a = H::default();
        a.write(b"hello world");
        let mut b = H::default();
        b.write(b"goodbye world");
        assert_ne!(a.finish(), b.finish());
    }

    #[test]
    fn ahash() {
        hashes_are_consistent::<ahash::AHasher>();
        hashes_differ_on_different_input::<ahash::AHasher>();
    }

    #[test]
    fn highway() {
        hashes_are_consistent::<highway::HighwayHasher>();
        hashes_differ_on_different_input::<highway::HighwayHasher>();
    }

    #[test]
    fn metrohash() {
        hashes_are_consistent::<metrohash::MetroHash128>();
        hashes_differ_on_different_input::<metrohash::MetroHash128>();
    }

    #[test]
    fn seahash() {
        hashes_are_consistent::<seahash::SeaHasher>();
        hashes_differ_on_different_input::<seahash::SeaHasher>();
    }

    #[test]
    fn twox_hash() {
        hashes_are_consistent::<twox_hash::xxhash3_128::Hasher>();
        hashes_differ_on_different_input::<twox_hash::xxhash3_128::Hasher>();
    }

    #[test]
    fn blake3() {
        hashes_are_consistent::<blake3::Hasher>();
        hashes_differ_on_different_input::<blake3::Hasher>();
    }
}
