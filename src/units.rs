//! Units of measure, so that byte counts stop being bare `u64`s as they
//! travel between the walker, the prefetcher and the hashing passes.

/// A count of bytes: a file's size, a read length, an offset.
///
/// Deliberately a thin wrapper: it exists to keep a file size from being
/// silently used where a file count or a thread count is expected, not to
/// provide arithmetic beyond what the pipeline actually needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bytes(u64);

impl Bytes {
    pub const fn new(count: u64) -> Self {
        Self(count)
    }

    pub const fn kib(count: u64) -> Self {
        Self(count * 1024)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    /// Saturating conversion, for use as a buffer length. Only ever called
    /// on values already clamped to a block size, so the saturation is a
    /// formality on 32-bit targets rather than a real case.
    pub fn as_usize(self) -> usize {
        usize::try_from(self.0).unwrap_or(usize::MAX)
    }

    pub fn min(self, other: Self) -> Self {
        Self(self.0.min(other.0))
    }

    /// Little-endian encoding, for feeding a size into a hasher.
    pub const fn to_le_bytes(self) -> [u8; 8] {
        self.0.to_le_bytes()
    }
}

impl From<u64> for Bytes {
    fn from(count: u64) -> Self {
        Self(count)
    }
}

/// Saturating: the only subtraction the pipeline performs is
/// `size - read_len` where `read_len <= size`, so an underflow would be a
/// bug, and clamping to zero keeps it from becoming a panic in a worker.
impl std::ops::Sub for Bytes {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}
