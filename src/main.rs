#[cfg(not(tarpaulin_include))]
mod args;

use byte_unit::Byte;
use std::io;
use std::path::PathBuf;
use structopt::clap::arg_enum;
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
        Format::Csv => csv_to_writer(io::stdout(), &counter.duplicates()).unwrap(),
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

fn csv_to_writer<W: std::io::Write>(
    writer: W,
    duplicates: &yadf::Duplicates<u64, yadf::DirEntry>,
) -> csv::Result<()> {
    use serde::ser::{Serialize, SerializeStruct, Serializer};
    struct Line<'a> {
        count: usize,
        files: &'a [yadf::DirEntry],
    }
    impl Serialize for Line<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            let mut state = serializer.serialize_struct("Line", 2)?;
            state.serialize_field("count", &self.count)?;
            state.serialize_field("files", &self.files)?;
            state.end()
        }
    }
    let mut writer = csv::WriterBuilder::new().flexible(true).from_writer(writer);
    for bucket in duplicates.iter() {
        writer.serialize(Line {
            count: bucket.len(),
            files: bucket,
        })?;
    }
    Ok(())
}
