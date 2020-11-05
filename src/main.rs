#[cfg(not(tarpaulin_include))]
mod args;

use byte_unit::Byte;
use std::io;
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
    /// check files with a name matching a PCRE regex
    ///
    /// almost PCRE, see: https://docs.rs/regex/1.4.2/regex/struct.Regex.html
    #[structopt(short = "R", long)]
    regex: Option<regex::Regex>,
    /// check files with a name matching a glob pattern
    ///
    /// see: https://docs.rs/globset/0.4.6/globset/index.html#syntax
    #[structopt(short, long, value_name = "glob")]
    pattern: Option<globset::Glob>,
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
    Highway,
    SeaHash,
    XxHash,
}

#[cfg(not(tarpaulin_include))]
fn main() {
    let args = Args::init_from_env();
    log::debug!("started with {:?}", args);
    let (min, max) = args.file_constraints();
    let config = yadf::Config::builder()
        .paths(&args.paths)
        .min(min)
        .max(max)
        .regex(args.regex.clone())
        .glob(args.pattern.clone().map(|g| g.compile_matcher()))
        .build();
    let counter = match args.algorithm {
        Algorithm::SeaHash => config.find_dupes::<yadf::SeaHasher>(),

        Algorithm::XxHash => config.find_dupes::<yadf::XxHasher>(),
        Algorithm::Highway => config.find_dupes::<yadf::HighwayHasher>(),
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
