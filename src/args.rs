use byte_unit::Byte;
use std::fmt;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

/// Yet Another Dupes Finder
#[derive(StructOpt, Debug)]
pub struct Args {
    /// directory to search
    #[structopt(default_value = ".")]
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

#[derive(Debug)]
pub enum Format {
    Fdupes,
    Json,
    JsonPretty,
    Machine,
}

#[derive(Debug)]
pub enum Algorithm {
    Highway,
    SeaHash,
    XxHash,
}

impl Args {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn format(&self) -> &Format {
        &self.format
    }

    pub fn report(&self) -> bool {
        self.report
    }

    pub fn algorithm(&self) -> &Algorithm {
        &self.algorithm
    }

    pub fn file_constraints(&self) -> Option<(u64, u64)> {
        Some((
            self.min
                .map(|m| m.get_bytes() as _)
                .unwrap_or(if self.no_empty { 1 } else { 0 }),
            self.max.map(|m| m.get_bytes() as _).unwrap_or(u64::MAX),
        ))
    }
}

impl Default for Format {
    fn default() -> Self {
        Self::Fdupes
    }
}

#[cfg(target_feature = "avx2")]
impl Default for Algorithm {
    fn default() -> Self {
        Self::Highway
    }
}

#[cfg(not(target_feature = "avx2"))]
impl Default for Algorithm {
    fn default() -> Self {
        Self::XxHash
    }
}

impl Format {
    const FDUPES: &'static str = "fdupes";
    const JSON: &'static str = "json";
    const JSON_PRETTY: &'static str = "json_pretty";
    const MACHINE: &'static str = "machine";
}

impl fmt::Display for Format {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let out = match self {
            Self::Fdupes => Self::FDUPES,
            Self::Json => Self::JSON,
            Self::JsonPretty => Self::JSON_PRETTY,
            Self::Machine => Self::MACHINE,
        };
        f.write_str(out)
    }
}

impl std::str::FromStr for Format {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::FDUPES => Ok(Self::Fdupes),
            Self::JSON => Ok(Self::Json),
            Self::JSON_PRETTY => Ok(Self::JsonPretty),
            Self::MACHINE => Ok(Self::Machine),
            _ => Err(format!(
                "can only be [standard|json|json_pretty|machine|fdupes], found: {:?}",
                s
            )),
        }
    }
}

impl Algorithm {
    const HIGHWAY: &'static str = "highway";
    const SEAHASH: &'static str = "seahash";
    const XXHASH: &'static str = "xxhash";
}

impl fmt::Display for Algorithm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Highway => f.write_str(Self::HIGHWAY),
            Self::SeaHash => f.write_str(Self::SEAHASH),
            Self::XxHash => f.write_str(Self::XXHASH),
        }
    }
}

impl std::str::FromStr for Algorithm {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            Self::HIGHWAY => Ok(Self::Highway),
            Self::SEAHASH => Ok(Self::SeaHash),
            Self::XXHASH => Ok(Self::XxHash),
            _ => Err(format!(
                "can only be [seahash|xxhash|highway], found: {:?}",
                s
            )),
        }
    }
}
