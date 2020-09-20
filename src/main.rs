#[cfg(not(tarpaulin_include))]
mod args;

use args::{Algorithm, Format};
use byte_unit::Byte;
use highway::HighwayHasher;
use seahash::SeaHasher;
use std::io;
use std::path::{Path, PathBuf};
use structopt::StructOpt;
use twox_hash::XxHash64;
use yadf::{Fdupes, Machine, Report};

/// Yet Another Dupes Finder
#[derive(StructOpt, Debug, Default)]
pub struct Args {
    /// Directories to search
    #[structopt(parse(from_os_str))]
    paths: Vec<PathBuf>,
    /// output format
    ///
    /// `standard`, `json`, `json_pretty`, `fdupes`, or `machine`
    #[structopt(short, long, default_value)]
    format: Format,
    /// Prints human readable report to stderr
    #[structopt(short, long)]
    report: bool,
    /// hashing algorithm
    ///
    /// `highway`, `seahash`, or `xxhash`
    #[structopt(short, long, default_value)]
    algorithm: Algorithm,
    /// Exclude empty files
    #[structopt(short, long)]
    no_empty: bool,
    /// minimum file size [default: no minimum]
    ///
    /// accepts standard formats
    #[structopt(long)]
    min: Option<Byte>,
    /// maximum file size [default: no maximum]
    ///
    /// accepts standard formats
    #[structopt(long)]
    max: Option<Byte>,
}

#[cfg(not(tarpaulin_include))]
fn main() {
    let args = Args::from_args();
    let (min, max) = args.file_constraints();
    let paths = normalize(&args.paths);
    let counter = match args.algorithm {
        Algorithm::SeaHash => yadf::find_dupes::<SeaHasher, PathBuf>(&paths, min, max),
        Algorithm::XxHash => yadf::find_dupes::<XxHash64, PathBuf>(&paths, min, max),
        Algorithm::Highway => yadf::find_dupes::<HighwayHasher, PathBuf>(&paths, min, max),
    };
    match args.format {
        Format::Json => {
            serde_json::to_writer(io::stdout(), &counter).unwrap();
            println!();
        }
        Format::JsonPretty => {
            serde_json::to_writer_pretty(io::stdout(), &counter).unwrap();
            println!();
        }
        Format::Fdupes => print!("{}", counter.display::<Fdupes>()),
        Format::Machine => print!("{}", counter.display::<Machine>()),
    };
    if args.report {
        let report = Report::from(&counter);
        eprintln!("{}", report);
    }
}

fn normalize<P: AsRef<Path>>(paths: &[P]) -> Vec<PathBuf> {
    use std::collections::HashSet;
    if paths.is_empty() {
        ["."].iter().map(Into::into).collect()
    } else {
        paths
            .iter()
            .map(AsRef::as_ref)
            .map(Into::into)
            .collect::<HashSet<PathBuf>>()
            .into_iter()
            .collect()
    }
}
