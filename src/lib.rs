//! A collection of functions and structs to find duplicate files.
//!
//! # Example :
//!
//! Find, display, and report, all the duplicate files at the given path :
//!
//! ```no_run
//! let hasher: std::marker::PhantomData<twox_hash::XxHash64> = Default::default();
//! let paths = ["."];
//! let files_counter = yadf::find_dupes(hasher, &paths, None, None);
//! println!("{}", files_counter.display::<yadf::Fdupes>());
//! eprintln!("{}", yadf::Report::from(&files_counter));
//! ```

mod bag;
pub mod fs;
mod report;

pub use bag::{Fdupes, Machine, TreeBag};
pub use fs::wrapper::DirEntry;
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
    P: AsRef<Path>,
{
    fs::dedupe::<H>(fs::find_dupes_partial::<H, P>(directories, min, max))
}
