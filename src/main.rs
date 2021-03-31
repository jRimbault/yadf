#![deny(unsafe_code)]
#![warn(rust_2018_idioms)]

mod args;

use anyhow::Context;
use byte_unit::Byte;
use std::fs::File;
use std::io::{self, Write};
use std::path::PathBuf;
use structopt::clap::arg_enum;
use yadf::{Fdupes, Machine};

fn main() -> anyhow::Result<()> {
    human_panic::setup_panic!();
    let timer = std::time::Instant::now();
    let args = Args::init_from_env();
    log::debug!("{:?}", args);
    let config = build_config(&args);
    log::debug!("{:?}", config);
    let bag = args.algorithm.run(config);
    let rfactor = args.rfactor.unwrap_or_default();
    let replicates = bag.replicates(rfactor.into());
    match args.output {
        Some(path) => {
            let context = || format!("writing output to the file: {:?}", path.display());
            let file = File::create(&path).with_context(context)?;
            args.format.display(file, replicates)
        }
        None => args.format.display(io::stdout().lock(), replicates),
    }
    .context("writing output")?;
    log::debug!("{:?} elapsed", timer.elapsed());
    Ok(())
}

#[cfg(unix)]
fn build_config(args: &Args) -> yadf::Yadf<PathBuf> {
    yadf::Yadf::builder()
        .paths(args.paths.as_ref())
        .minimum_file_size(args.min())
        .maximum_file_size(args.max())
        .regex(args.regex.clone())
        .glob(args.pattern.clone())
        .max_depth(args.max_depth)
        .hard_links(args.hard_links)
        .build()
}

#[cfg(not(unix))]
fn build_config(args: &Args) -> yadf::Yadf<PathBuf> {
    yadf::Yadf::builder()
        .paths(args.paths.as_ref())
        .minimum_file_size(args.min())
        .maximum_file_size(args.max())
        .regex(args.regex.clone())
        .glob(args.pattern.clone())
        .max_depth(args.max_depth)
        .build()
}

impl Algorithm {
    fn run<P>(&self, config: yadf::Yadf<P>) -> yadf::FileCounter
    where
        P: AsRef<std::path::Path>,
    {
        log::debug!("using {} hashing", self);
        match self {
            Algorithm::AHash => config.scan::<ahash::AHasher>(),
            Algorithm::Highway => config.scan::<highway::HighwayHasher>(),
            Algorithm::MetroHash => config.scan::<metrohash::MetroHash>(),
            Algorithm::SeaHash => config.scan::<seahash::SeaHasher>(),
            Algorithm::XxHash => config.scan::<twox_hash::XxHash64>(),
        }
    }
}

impl Format {
    fn display<W>(&self, writer: W, replicates: yadf::FileReplicates<'_>) -> anyhow::Result<()>
    where
        W: Write,
    {
        let mut writer = io::BufWriter::with_capacity(64 * 1024, writer);
        match self {
            Format::Json => {
                serde_json::to_writer(&mut writer, &replicates)?;
                writer.write_all(b"\n")?;
            }
            Format::JsonPretty => {
                serde_json::to_writer_pretty(&mut writer, &replicates)?;
                writer.write_all(b"\n")?;
            }
            Format::Csv => csv_to_writer(writer, &replicates)?,
            Format::LdJson => ldjson_to_writer(writer, &replicates)?,
            Format::Fdupes => writeln!(writer, "{}", replicates.display::<Fdupes>())?,
            Format::Machine => writeln!(writer, "{}", replicates.display::<Machine>())?,
        };
        Ok(())
    }
}

/// Yet Another Dupes Finder
#[derive(structopt::StructOpt, Debug)]
// #[structopt(setting(structopt::clap::AppSettings::ColoredHelp))]
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
    /// Treat hard links to same file as duplicates
    #[cfg(unix)]
    #[cfg_attr(unix, structopt(short = "H", long))]
    hard_links: bool,
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
    /// Optional output file
    #[structopt(short, long, parse(from_os_str))]
    output: Option<PathBuf>,
}

arg_enum! {
    #[derive(Debug)]
    enum Format {
        Csv,
        Fdupes,
        Json,
        JsonPretty,
        LdJson,
        Machine,
    }
}

arg_enum! {
    #[derive(Debug)]
    enum Algorithm {
        AHash,
        Highway,
        MetroHash,
        SeaHash,
        XxHash,
    }
}

#[derive(Debug, PartialEq)]
enum ReplicationFactor {
    Under(usize),
    Equal(usize),
    Over(usize),
}

/// mimic serde_json interface
fn csv_to_writer<W>(writer: W, replicates: &yadf::FileReplicates<'_>) -> csv::Result<()>
where
    W: Write,
{
    let mut writer = csv::WriterBuilder::new()
        .flexible(true)
        .has_headers(false)
        .from_writer(writer);
    writer.serialize(("count", "files"))?;
    for files in replicates {
        writer.serialize((files.len(), files))?;
    }
    Ok(())
}

/// mimic serde_json interface
fn ldjson_to_writer<W>(mut writer: W, replicates: &yadf::FileReplicates<'_>) -> anyhow::Result<()>
where
    W: Write,
{
    for files in replicates {
        serde_json::to_writer(&mut writer, &files)?;
        writeln!(writer)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;

    static BAG: Lazy<yadf::TreeBag<u64, yadf::Path>> = Lazy::new(|| {
        vec![
            (77, "hello".into()),
            (77, "world".into()),
            (3, "foo".into()),
            (3, "bar".into()),
        ]
        .into_iter()
        .collect()
    });

    #[test]
    fn csv() {
        let mut buffer = Vec::new();
        let _ = csv_to_writer(&mut buffer, &BAG.duplicates());
        let result = String::from_utf8(buffer).unwrap();
        let expected = r#"count,files
2,foo,bar
2,hello,world
"#;
        assert_eq!(result, expected);
    }

    #[test]
    fn ldjson() {
        let mut buffer = Vec::new();
        let _ = ldjson_to_writer(&mut buffer, &BAG.duplicates());
        let result = String::from_utf8(buffer).unwrap();
        let expected = r#"["foo","bar"]
["hello","world"]
"#;
        assert_eq!(result, expected);
    }
}
