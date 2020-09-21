#[cfg(not(tarpaulin_include))]
mod args;

use args::{Algorithm, Format};
use byte_unit::Byte;
use highway::HighwayHasher;
use seahash::SeaHasher;
use std::env;
use std::io;
use std::path::PathBuf;
use structopt::StructOpt;
use twox_hash::XxHash64;
use yadf::{Fdupes, Machine, Report};

/// Yet Another Dupes Finder
#[derive(StructOpt, Debug, Default)]
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
    ///
    /// accepts standard formats: K, M, G, T, P
    #[structopt(long)]
    min: Option<Byte>,
    /// maximum file size [default: no maximum]
    ///
    /// accepts standard formats: K, M, G, T, P
    #[structopt(long)]
    max: Option<Byte>,
}

#[cfg(not(tarpaulin_include))]
fn main() {
    let args = Args::from_args();
    let cwd = env::current_dir().expect("couldn't get current working directory");
    let (min, max) = args.file_constraints();
    let counter = match args.algorithm {
        Algorithm::SeaHash => yadf::find_dupes::<SeaHasher, PathBuf>(&args.paths(cwd), min, max),
        Algorithm::XxHash => yadf::find_dupes::<XxHash64, PathBuf>(&args.paths(cwd), min, max),
        Algorithm::Highway => {
            yadf::find_dupes::<HighwayHasher, PathBuf>(&args.paths(cwd), min, max)
        }
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
