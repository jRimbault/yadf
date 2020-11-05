//! A collection of functions and structs to find duplicate files.
//!
//! # Example :
//!
//! Find, display, and report, all the duplicate files at the given path :
//!
//! ```no_run
//! let counter = yadf::Config::builder().paths(&["."]).build().find_dupes::<yadf::SeaHasher>();
//! println!("{}", counter.duplicates().display::<yadf::Fdupes>());
//! eprintln!("{}", yadf::Report::from(&counter));
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

/// Search configuration
#[derive(Debug, Default, typed_builder::TypedBuilder)]
pub struct Config<'a, P>
where
    P: AsRef<Path>,
{
    paths: &'a [P],
    #[builder(default)]
    min: Option<u64>,
    #[builder(default)]
    max: Option<u64>,
    #[builder(default)]
    regex: Option<regex::Regex>,
}

impl<P> Config<'_, P>
where
    P: AsRef<Path>,
{
    /// This will attemps a complete scan of every file,
    /// within the given size constraints, at the given path.
    pub fn find_dupes<H>(self) -> TreeBag<u64, DirEntry>
    where
        H: Hasher + Default,
        H: std::io::Write,
    {
        let dupes = fs::find_dupes_partial::<H, P>(self.paths, self.min, self.max, self.regex);
        if log::log_enabled!(log::Level::Info) {
            log::info!(
                "scanned {} files",
                dupes.values().map(|b| b.len()).sum::<usize>()
            );
            log::info!(
                "found {} possible duplicates after initial scan",
                dupes.duplicates().iter().map(|b| b.len()).sum::<usize>()
            );
            if log::log_enabled!(log::Level::Debug) {
                log::debug!("{:?}", dupes);
            }
        }
        let dupes = fs::dedupe::<H>(dupes);
        if log::log_enabled!(log::Level::Info) {
            log::info!(
                "found {} duplicates in {} groups after checksumming",
                dupes.duplicates().iter().map(|b| b.len()).sum::<usize>(),
                dupes.duplicates().iter().count(),
            );
            if log::log_enabled!(log::Level::Debug) {
                log::debug!("{:?}", dupes);
            }
        }
        dupes
    }
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
