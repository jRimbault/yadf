//! This a binary crate. You _can_ use it as a library, but I wouldn't recommend it.
//!
//! A collection of functions and structs to find duplicate files.
//!
//! # Example :
//!
//! Find and display all the duplicate files at the given path :
//!
//! ```no_run
//! let counter = yadf::Yadf::builder()
//!     .paths(&["path/to/somewhere", "another/path"]) // required
//!     .minimum_file_size(64) // optional
//!     .maximum_file_size(1024 * 8) // optional
//!     .regex(None) // optional
//!     .glob(None) // optional
//!     .build()
//!     .scan::<highway::HighwayHasher>();
//! println!("{}", counter.duplicates().display::<yadf::Fdupes>());
//! ```

mod bag;
mod fs;

pub use bag::{Factor, Fdupes, Machine, Replicates, TreeBag};
pub use globset;
pub use regex;
use std::path::{Path, PathBuf};

/// Meta trait for the Hasher and Default traits
pub trait Hasher: core::hash::Hasher + core::default::Default {}
impl<T> Hasher for T
where
    T: core::hash::Hasher,
    T: core::default::Default,
{
}

/// Search configuration
///
/// # Example
///
/// ```no_run
/// let counter = yadf::Yadf::builder()
///     .paths(&["path/to/somewhere", "another/path"]) // required
///     .minimum_file_size(64) // optional
///     .maximum_file_size(1024 * 8) // optional
///     .regex(None) // optional
///     .glob(None) // optional
///     .build()
///     .scan::<highway::HighwayHasher>();
/// ```
///
/// see the docs for the [`YadfBuilder`](YadfBuilder)
#[derive(Debug, Default, typed_builder::TypedBuilder)]
#[builder(doc)]
pub struct Yadf<'a, P>
where
    P: AsRef<Path>,
{
    #[builder(setter(doc = "Paths that will be checked for duplicate files"))]
    paths: &'a [P],
    #[builder(default, setter(into, doc = "Minimum file size"))]
    minimum_file_size: Option<u64>,
    #[builder(default, setter(into, doc = "Maximum file size"))]
    maximum_file_size: Option<u64>,
    #[builder(default, setter(into, doc = "Maximum recursion depth"))]
    max_depth: Option<usize>,
    #[builder(default, setter(into, doc = "File name must match this regex"))]
    regex: Option<regex::Regex>,
    #[builder(default, setter(into, doc = "File name must match this glob"))]
    glob: Option<globset::Glob>,
}

impl<P> Yadf<'_, P>
where
    P: AsRef<Path>,
{
    /// This will attemps a complete scan according to its configuration.
    pub fn scan<H: Hasher>(self) -> TreeBag<u64, PathBuf> {
        let bag = fs::find_dupes_partial::<H, P>(
            self.paths,
            self.minimum_file_size,
            self.maximum_file_size,
            self.regex,
            self.glob.map(|g| g.compile_matcher()),
            self.max_depth,
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
            log::trace!("{:?}", bag);
        }
        let bag = fs::dedupe::<H>(bag);
        if log::log_enabled!(log::Level::Info) {
            log::info!(
                "found {} duplicates in {} groups after checksumming",
                bag.duplicates().iter().map(|b| b.len()).sum::<usize>(),
                bag.duplicates().iter().count(),
            );
            log::trace!("{:?}", bag);
        }
        bag
    }
}
