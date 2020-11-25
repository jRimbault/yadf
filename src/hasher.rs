#[macro_export]
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

#[macro_export]
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

#[macro_export]
macro_rules! newtype_impl_hasher_and_write {
    ($hasher:ty) => {
        $crate::newtype_impl_hasher!($hasher);
        $crate::newtype_impl_write!($hasher);
    };
}
/// Hasher struct implementing Hasher, Default and Write
#[derive(Default)]
#[repr(transparent)]
pub struct HighwayHasher(highway::HighwayHasher);
/// Hasher struct implementing Hasher, Default and Write
#[derive(Default)]
#[repr(transparent)]
pub struct SeaHasher(seahash::SeaHasher);
/// Hasher struct implementing Hasher, Default and Write
#[derive(Default)]
#[repr(transparent)]
pub struct XxHasher(twox_hash::XxHash64);

newtype_impl_hasher_and_write!(HighwayHasher);
newtype_impl_hasher_and_write!(SeaHasher);
newtype_impl_hasher_and_write!(XxHasher);
