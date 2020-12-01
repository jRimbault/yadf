macro_rules! newtype_impl_write {
    ($hasher:ty) => {
        impl ::std::io::Write for $hasher {
            fn write(&mut self, buf: &[u8]) -> ::std::io::Result<usize> {
                ::core::hash::Hasher::write(&mut self.0, buf);
                Ok(buf.len())
            }
            fn flush(&mut self) -> ::std::io::Result<()> {
                Ok(())
            }
        }
    };
}

macro_rules! newtype_impl_hasher {
    ($hasher:ty) => {
        impl ::core::hash::Hasher for $hasher {
            fn write(&mut self, buf: &[u8]) {
                ::core::hash::Hasher::write(&mut self.0, buf)
            }
            fn finish(&self) -> u64 {
                ::core::hash::Hasher::finish(&self.0)
            }
        }
    };
}

/// Hasher struct implementing Hasher, Default and Write
#[derive(Default)]
#[repr(transparent)]
pub struct SeaHasher(seahash::SeaHasher);
newtype_impl_hasher!(SeaHasher);
newtype_impl_write!(SeaHasher);

/// Hasher struct implementing Hasher, Default and Write
#[derive(Default)]
#[repr(transparent)]
pub struct XxHasher(twox_hash::XxHash64);
newtype_impl_hasher!(XxHasher);
newtype_impl_write!(XxHasher);

/// Hasher struct implementing Hasher, Default and Write
#[derive(Default)]
#[repr(transparent)]
pub struct MetroHash(metrohash::MetroHash64);
newtype_impl_hasher!(MetroHash);
newtype_impl_write!(MetroHash);
