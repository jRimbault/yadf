#[cfg(not(tarpaulin_include))]
mod args;

use byte_unit::Byte;
use std::io;
use std::marker;
use std::path::PathBuf;
use yadf::{Fdupes, Machine, Report};

/// Yet Another Dupes Finder
#[derive(structopt::StructOpt, Debug)]
#[structopt(setting(structopt::clap::AppSettings::ColoredHelp))]
pub struct Args {
    /// Directories to search
    ///
    /// default is to search inside the current working directory
    #[structopt(parse(from_os_str))]
    paths: Vec<PathBuf>,
    /// output format
    ///
    /// `json`, `json_pretty`, `fdupes`, or `machine`
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
    #[structopt(long, value_name = "size")]
    min: Option<Byte>,
    /// maximum file size [default: no maximum]
    #[structopt(long, value_name = "size")]
    max: Option<Byte>,
    /// only matching files with a name matching a PCRE regex
    #[structopt(short, long, value_name = "regex")]
    pattern: Option<regex::Regex>,
    #[structopt(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
enum Format {
    Fdupes,
    Json,
    JsonPretty,
    Machine,
}

#[derive(Debug)]
#[cfg_attr(test, derive(PartialEq))]
enum Algorithm {
    Highway(marker::PhantomData<yadf::HighwayHasher>),
    SeaHash(marker::PhantomData<yadf::SeaHasher>),
    XxHash(marker::PhantomData<yadf::XxHasher>),
}

#[cfg(not(tarpaulin_include))]
fn main() {
    use Algorithm::*;
    let args = Args::from_env();
    init_logger(&args);
    log::debug!("started with {:?}", args);
    let (min, max) = args.file_constraints();
    let counter = match args.algorithm {
        SeaHash(hasher) => yadf::find_dupes(hasher, &args.paths, min, max, args.pattern.clone()),
        XxHash(hasher) => yadf::find_dupes(hasher, &args.paths, min, max, args.pattern.clone()),
        Highway(hasher) => yadf::find_dupes(hasher, &args.paths, min, max, args.pattern.clone()),
    };
    match args.format {
        Format::Json => serde_json::to_writer(io::stdout(), &counter.duplicates()).unwrap(),
        Format::JsonPretty => {
            serde_json::to_writer_pretty(io::stdout(), &counter.duplicates()).unwrap()
        }
        Format::Fdupes => print!("{}", counter.duplicates().display::<Fdupes>()),
        Format::Machine => print!("{}", counter.duplicates().display::<Machine>()),
    };
    println!();
    if args.report {
        let report = Report::from(&counter);
        eprintln!("{}", report);
    }
}

#[cfg(not(tarpaulin_include))]
fn init_logger(args: &Args) {
    env_logger::Builder::new()
        .filter_level(
            args.verbosity
                .log_level()
                .unwrap_or(log::Level::Error)
                .to_level_filter(),
        )
        .init();
}
