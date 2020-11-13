//! This a binary crate. You _can_ use it as a library, but I wouldn't recommend it.
//!
//! A collection of functions and structs to find duplicate files.
//!
//! # Example :
//!
//! Find, display, and report, all the duplicate files at the given path :
//!
//! ```no_run
//! let counter = yadf::Config::builder()
//!     .paths(&["path/to/somewhere", "another/path"]) // required
//!     .minimum_file_size(64) // optional
//!     .maximum_file_size(1024 * 8) // optional
//!     .regex(None) // optional
//!     .glob(None) // optional
//!     .build()
//!     .scan::<yadf::HighwayHasher>();
//! println!("{}", counter.duplicates().display::<yadf::Fdupes>());
//! eprintln!("{}", yadf::Report::from(&counter));
//! ```

mod bag;
mod fs;
mod macros;
mod report;

pub use bag::{Duplicates, Fdupes, Machine, TreeBag};
pub use fs::wrapper::DirEntry;
pub use globset;
#[cfg(any(test, feature = "build-bin"))]
pub use hashers::{HighwayHasher, SeaHasher, XxHasher};
pub use regex;
pub use report::Report;
use std::path::Path;

/// Meta trait for the Hasher, Default and Write traits
pub trait Hasher: core::hash::Hasher + std::io::Write + core::default::Default {}
impl<T> Hasher for T
where
    T: core::hash::Hasher,
    T: core::default::Default,
    T: std::io::Write,
{
}

/// Search configuration
///
/// # Example
///
/// ```no_run
/// let counter = yadf::Config::builder()
///     .paths(&["path/to/somewhere", "another/path"]) // required
///     .minimum_file_size(64) // optional
///     .maximum_file_size(1024 * 8) // optional
///     .regex(None) // optional
///     .glob(None) // optional
///     .build()
///     .scan::<yadf::HighwayHasher>();
/// ```
///
/// see the docs for the [ConfigBuilder](struct.ConfigBuilder.html)
#[derive(Debug, Default, typed_builder::TypedBuilder)]
#[builder(doc)]
pub struct Config<'a, P>
where
    P: AsRef<Path>,
{
    #[builder(setter(doc = "Paths that will be checked for duplicate files"))]
    paths: &'a [P],
    #[builder(default, setter(into, doc = "Minimum file size"))]
    minimum_file_size: Option<u64>,
    #[builder(default, setter(into, doc = "Maximum file size"))]
    maximum_file_size: Option<u64>,
    #[builder(default, setter(into, doc = "File name must match this regex"))]
    regex: Option<regex::Regex>,
    #[builder(default, setter(into, doc = "File name must match this glob"))]
    glob: Option<globset::Glob>,
}

impl<P> Config<'_, P>
where
    P: AsRef<Path>,
{
    /// This will attemps a complete scan according to its configuration.
    pub fn scan<H: Hasher>(self) -> TreeBag<u64, DirEntry> {
        let bag = fs::find_dupes_partial::<H, P>(
            self.paths,
            self.minimum_file_size,
            self.maximum_file_size,
            self.regex,
            self.glob.map(|g| g.compile_matcher()),
        );
        if log::log_enabled!(log::Level::Info) {
            log::info!(
                "scanned {} files",
                bag.0.values().map(|b| b.len()).sum::<usize>()
            );
            log::info!(
                "found {} possible duplicates after initial scan",
                bag.duplicates().iter().map(|b| b.len()).sum::<usize>()
            );
            if log::log_enabled!(log::Level::Debug) {
                log::debug!("{:?}", bag);
            }
        }
        let dupes = fs::dedupe::<H>(bag);
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

    super::newtype_impl_hasher_and_write!(HighwayHasher);
    super::newtype_impl_hasher_and_write!(SeaHasher);
    super::newtype_impl_hasher_and_write!(XxHasher);
}
