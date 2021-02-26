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
#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

mod bag;
mod fs;
pub mod path;

pub use bag::{Factor, Fdupes, Machine, Replicates, TreeBag};
pub use globset;
pub use regex;
use std::hash::Hasher;
use std::path::Path;

pub type FileCounter = TreeBag<u64, path::Path>;
pub type FileReplicates<'a> = Replicates<'a, u64, path::Path>;

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
pub struct Yadf<'a, P> {
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
    #[cfg(unix)]
    #[builder(default, setter(doc = "Treat hard links as duplicates"))]
    hard_links: bool,
}

impl<P> Yadf<'_, P>
where
    P: AsRef<Path>,
{
    /// This will attemps a complete scan according to its configuration.
    pub fn scan<H>(self) -> FileCounter
    where
        H: Hasher + Default,
    {
        #[cfg(unix)]
        let file_filter = fs::filter::FileFilter::new(
            self.minimum_file_size,
            self.maximum_file_size,
            self.regex,
            self.glob.map(|g| g.compile_matcher()),
            self.hard_links,
        );
        #[cfg(not(unix))]
        let file_filter = fs::filter::FileFilter::new(
            self.minimum_file_size,
            self.maximum_file_size,
            self.regex,
            self.glob.map(|g| g.compile_matcher()),
        );
        let bag = fs::find_dupes_partial::<H, P>(self.paths, self.max_depth, file_filter);
        if log::log_enabled!(log::Level::Info) {
            log::info!(
                "scanned {} files",
                bag.0.values().map(Vec::len).sum::<usize>()
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
