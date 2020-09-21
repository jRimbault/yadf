#[cfg(not(tarpaulin_include))]
mod args;

use byte_unit::Byte;
use highway::HighwayHasher;
use seahash::SeaHasher;
use std::io;
use std::path::PathBuf;
use twox_hash::XxHash64;
use yadf::{Fdupes, Machine, Report};

/// Yet Another Dupes Finder
///
/// For sizes, K/M/G/T[B|iB] suffixes can be used (case-insensitive)
#[derive(structopt::StructOpt, Debug, Default)]
pub struct Args {
    /// Directories to search
    ///
    /// default is to search inside the current working directory
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
    /// Excludes empty files
    #[structopt(short, long)]
    no_empty: bool,
    /// minimum file size [default: no minimum]
    #[structopt(long)]
    min: Option<Byte>,
    /// maximum file size [default: no maximum]
    #[structopt(long)]
    max: Option<Byte>,
}

#[derive(Debug)]
enum Format {
    Fdupes,
    Json,
    JsonPretty,
    Machine,
}

#[derive(Debug)]
enum Algorithm {
    Highway,
    SeaHash,
    XxHash,
}

#[cfg(not(tarpaulin_include))]
fn main() {
    let args = Args::from_env();
    let (min, max) = args.file_constraints();
    let counter = match args.algorithm {
        Algorithm::SeaHash => yadf::find_dupes::<SeaHasher, PathBuf>(&args.paths, min, max),
        Algorithm::XxHash => yadf::find_dupes::<XxHash64, PathBuf>(&args.paths, min, max),
        Algorithm::Highway => yadf::find_dupes::<HighwayHasher, PathBuf>(&args.paths, min, max),
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
        Format::Fdupes => println!("{}", counter.display::<Fdupes>()),
        Format::Machine => println!("{}", counter.display::<Machine>()),
    };
    if args.report {
        let report = Report::from(&counter);
        eprintln!("{}", report);
    }
}
