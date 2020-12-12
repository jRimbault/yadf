mod args;

use byte_unit::Byte;
use std::io;
use std::path::PathBuf;
use structopt::clap::arg_enum;
use yadf::{Fdupes, Machine};

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
    /// Maximum recursion depth
    #[structopt(short = "d", long = "depth", value_name = "depth")]
    max_depth: Option<usize>,
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
    /// Replication factor [under|equal|over]:n
    ///
    /// The default is `over:1`, to find uniques use `equal:1`,
    /// to find files with less than 10 copies use `under:10`
    #[structopt(long)]
    rfactor: Option<ReplicationFactor>,
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
        MetroHash,
        SeaHash,
        XxHash,
    }
}

#[derive(Debug)]
enum ReplicationFactor {
    Under(usize),
    Equal(usize),
    Over(usize),
}

fn main() {
    human_panic::setup_panic!();
    let timer = std::time::Instant::now();
    let args = Args::init_from_env();
    log::debug!("{:?}", args);
    let config = yadf::Yadf::builder()
        .paths(&args.paths)
        .minimum_file_size(args.min())
        .maximum_file_size(args.max())
        .regex(args.regex.clone())
        .glob(args.pattern.clone())
        .max_depth(args.max_depth)
        .build();
    log::debug!("{:?}", config);
    let bag = match args.algorithm {
        Algorithm::Highway => config.scan::<highway::HighwayHasher>(),
        Algorithm::MetroHash => config.scan::<metrohash::MetroHash>(),
        Algorithm::SeaHash => config.scan::<seahash::SeaHasher>(),
        Algorithm::XxHash => config.scan::<twox_hash::XxHash64>(),
    };
    let replicates = bag.replicates(args.rfactor.unwrap_or_default().into());
    match args.format {
        Format::Json => {
            serde_json::to_writer(io::stdout(), &replicates).unwrap();
            println!();
        }
        Format::JsonPretty => {
            serde_json::to_writer_pretty(io::stdout(), &replicates).unwrap();
            println!();
        }
        Format::Csv => csv_to_writer(io::stdout(), &replicates).unwrap(),
        Format::Fdupes => println!("{}", replicates.display::<Fdupes>()),
        Format::Machine => println!("{}", replicates.display::<Machine>()),
    };
    log::debug!("{:?} elapsed", timer.elapsed());
}

/// mimic serde_json interface
fn csv_to_writer<W: std::io::Write>(
    writer: W,
    duplicates: &yadf::Replicates<u64, PathBuf>,
) -> csv::Result<()> {
    let mut writer = csv::WriterBuilder::new()
        .flexible(true)
        .has_headers(false)
        .from_writer(writer);
    writer.serialize(("count", "files"))?;
    for files in duplicates.iter() {
        writer.serialize((files.len(), files))?;
    }
    Ok(())
}
