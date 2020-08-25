#[cfg(not(tarpaulin_include))]
mod args;

use args::{Algorithm, Format};
use byte_unit::Byte;
use highway::HighwayHasher;
use seahash::SeaHasher;
use std::io;
use std::path::PathBuf;
use structopt::StructOpt;
use twox_hash::XxHash64;
use yadf::{Fdupes, Machine, Report};

/// Yet Another Dupes Finder
#[derive(StructOpt, Debug, Default)]
pub struct Args {
    /// directory to search
    #[structopt(default_value = ".", parse(from_os_str))]
    path: PathBuf,
    /// output format `standard`, `json`, `json_pretty`, `fdupes` or `machine`
    #[structopt(short, long, default_value)]
    format: Format,
    /// print human readable report to stderr
    #[structopt(short, long)]
    report: bool,
    /// hashing algorithm
    #[structopt(short, long, default_value)]
    algorithm: Algorithm,
    /// exclude empty files
    #[structopt(short, long)]
    no_empty: bool,
    /// minimum file size (default 0 byte)
    #[structopt(long)]
    min: Option<Byte>,
    /// maximum file size (default no maximum)
    #[structopt(long)]
    max: Option<Byte>,
}

#[cfg(not(tarpaulin_include))]
fn main() {
    let args = Args::from_args();
    let min_max_file_size = args.file_constraints();
    let counter = match args.algorithm {
        Algorithm::SeaHash => yadf::find_dupes::<SeaHasher>(&args.path, min_max_file_size),
        Algorithm::XxHash => yadf::find_dupes::<XxHash64>(&args.path, min_max_file_size),
        Algorithm::Highway => yadf::find_dupes::<HighwayHasher>(&args.path, min_max_file_size),
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
