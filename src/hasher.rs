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
