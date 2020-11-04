//! A collection of functions and structs to find duplicate files.
//!
//! # Example :
//!
//! Find, display, and report, all the duplicate files at the given path :
//!
//! ```no_run
//! let hasher: std::marker::PhantomData<yadf::XxHasher> = Default::default();
//! let paths = ["."];
//! let files_counter = yadf::find_dupes(hasher, &paths, None, None);
//! println!("{}", files_counter.duplicates().display::<yadf::Fdupes>());
//! eprintln!("{}", yadf::Report::from(&files_counter));
//! ```

mod bag;
pub mod fs;
mod macros;
mod report;

pub use bag::{Fdupes, Machine, TreeBag};
pub use fs::wrapper::DirEntry;
#[cfg(any(test, feature = "build-bin"))]
pub use hashers::{HighwayHasher, SeaHasher, XxHasher};
pub use report::Report;
use std::hash::Hasher;
use std::path::Path;

/// This will attemps a complete scan of every file,
/// within the given size constraints, at the given path.
pub fn find_dupes<H, P>(
    _hasher: std::marker::PhantomData<H>,
    directories: &[P],
    min: Option<u64>,
    max: Option<u64>,
) -> TreeBag<u64, DirEntry>
where
    H: Hasher + Default,
    H: std::io::Write,
    P: AsRef<Path>,
{
    let dupes = fs::find_dupes_partial::<H, P>(directories, min, max);
    if log::log_enabled!(log::Level::Info) {
        log::info!(
            "scanned {} files",
            dupes.values().map(|b| b.len()).sum::<usize>()
        );
        log::info!(
            "found {} possible duplicates after initial scan",
            dupes.duplicates().values().map(|b| b.len()).sum::<usize>()
        );
    }
    let dupes = fs::dedupe::<H>(dupes);
    if log::log_enabled!(log::Level::Info) {
        log::info!(
            "found {} duplicates in {} groups after checksumming",
            dupes.duplicates().values().map(|b| b.len()).sum::<usize>(),
            dupes.duplicates().values().count(),
        );
    }
    dupes
}

#[cfg(any(test, feature = "build-bin"))]
mod hashers {
    #[derive(Default)]
    #[repr(transparent)]
    pub struct HighwayHasher(highway::HighwayHasher);
    #[derive(Default)]
    #[repr(transparent)]
    pub struct SeaHasher(seahash::SeaHasher);
    #[derive(Default)]
    #[repr(transparent)]
    pub struct XxHasher(twox_hash::XxHash64);

    super::newtype_impl_hasher_and_write!(HighwayHasher);
    super::newtype_impl_hasher_and_write!(SeaHasher);
    super::newtype_impl_hasher_and_write!(XxHasher);
}
