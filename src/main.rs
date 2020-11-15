#[cfg(not(tarpaulin_include))]
mod args;

use byte_unit::Byte;
use std::io;
use std::path::PathBuf;
use structopt::clap::arg_enum;
use yadf::{Csv, Fdupes, Machine, Report};

/// Yet Another Dupes Finder
#[derive(structopt::StructOpt, Debug)]
#[structopt(setting(structopt::clap::AppSettings::ColoredHelp))]
pub struct Args {
    /// Directories to search
    ///
    /// default is to search inside the current working directory
    #[structopt(parse(from_os_str))]
    paths: Vec<PathBuf>,
    /// Output format
    #[structopt(
        short,
        long,
        default_value,
        possible_values = &Format::variants(),
        case_insensitive = true
    )]
    format: Format,
    /// Prints human readable report to stderr
    #[structopt(short, long)]
    report: bool,
    /// Hashing algorithm
    #[structopt(
        short,
        long,
        default_value,
        possible_values = &Algorithm::variants(),
        case_insensitive = true
    )]
    algorithm: Algorithm,
    /// Excludes empty files
    #[structopt(short, long)]
    no_empty: bool,
    /// Minimum file size
    #[structopt(long, value_name = "size")]
    min: Option<Byte>,
    /// Maximum file size
    #[structopt(long, value_name = "size")]
    max: Option<Byte>,
    /// Check files with a name matching a Perl-style regex,
    /// see: https://docs.rs/regex/1.4.2/regex/index.html#syntax
    #[structopt(short = "R", long)]
    regex: Option<regex::Regex>,
    /// Check files with a name matching a glob pattern,
    /// see: https://docs.rs/globset/0.4.6/globset/index.html#syntax
    #[structopt(short, long, value_name = "glob")]
    pattern: Option<globset::Glob>,
    #[structopt(flatten)]
    verbosity: clap_verbosity_flag::Verbosity,
}

arg_enum! {
    #[derive(Debug)]
    enum Format {
        Csv,
        Fdupes,
        Json,
        JsonPretty,
        Machine,
    }
}

arg_enum! {
    #[derive(Debug)]
    enum Algorithm {
        Highway,
        SeaHash,
        XxHash,
    }
}

#[cfg(not(tarpaulin_include))]
fn main() {
    human_panic::setup_panic!();
    let args = Args::init_from_env();
    log::debug!("started with {:?}", args);
    let config = yadf::Config::builder()
        .paths(&args.paths)
        .minimum_file_size(args.min())
        .maximum_file_size(args.max())
        .regex(args.regex.clone())
        .glob(args.pattern.clone())
        .build();
    log::debug!("config is {:?}", config);
    let counter = match args.algorithm {
        Algorithm::SeaHash => config.scan::<yadf::SeaHasher>(),
        Algorithm::XxHash => config.scan::<yadf::XxHasher>(),
        Algorithm::Highway => config.scan::<yadf::HighwayHasher>(),
    };
    match args.format {
        Format::Json => serde_json::to_writer(io::stdout(), &counter.duplicates()).unwrap(),
        Format::JsonPretty => {
            serde_json::to_writer_pretty(io::stdout(), &counter.duplicates()).unwrap()
        }
        Format::Csv => print!("{}", counter.duplicates().display::<Csv>()),
        Format::Fdupes => print!("{}", counter.duplicates().display::<Fdupes>()),
        Format::Machine => print!("{}", counter.duplicates().display::<Machine>()),
    };
    println!();
    if args.report {
        let report = Report::from(&counter);
        eprintln!("{}", report);
    }
}
