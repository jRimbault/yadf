//! A collection of functions and structs to find duplicate files.
//!
//! # Example :
//!
//! Find, display, and report, all the duplicate files at the given path :
//!
//! ```
//! let files_counter = yadf::find_dupes::<twox_hash::XxHash64>(".".as_ref(), None);
//! print!("{}", files_counter.display::<yadf::Fdupes>());
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
pub fn find_dupes<H>(dir: &Path, min_max_file_size: Option<(u64, u64)>) -> TreeBag<u64, DirEntry>
where
    H: Hasher + Default,
{
    fs::dedupe::<H>(fs::find_dupes_partial::<H>(dir, min_max_file_size))
}
