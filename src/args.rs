use super::{Algorithm, Args, Format};
use std::collections::HashSet;
use std::env;
use std::fmt;
use std::path::PathBuf;
use structopt::StructOpt;

impl Args {
    pub fn file_constraints(&self) -> (Option<u64>, Option<u64>) {
        (
            self.min
                .map(|m| m.get_bytes())
                .or(if self.no_empty { Some(1) } else { None }),
            self.max.map(|m| m.get_bytes()),
        )
    }

    /// returns a list of the deduped paths
    fn paths(&self, cwd: impl Fn() -> PathBuf) -> Vec<PathBuf> {
        if self.paths.is_empty() {
            vec![cwd()]
        } else {
            self.paths
                .iter()
                .cloned()
                .collect::<HashSet<PathBuf>>()
                .into_iter()
                .collect()
        }
    }

    #[cfg(not(tarpaulin_include))]
    pub fn from_env() -> Self {
        // Clap requires that every string we give it will be
        // 'static, but we need to build the version string dynamically.
        // We can fake the 'static lifetime with lazy_static.
        lazy_static::lazy_static! {
            static ref LONG_VERSION: String = env!("YADF_BUILD_VERSION").replace("|", "\n");
        }
        let app = Self::clap()
            .long_version(LONG_VERSION.as_str())
            .after_help("For sizes, K/M/G/T[B|iB] suffixes can be used (case-insensitive).");
        let mut args = Self::from_clap(&app.get_matches());
        let cwd = || env::current_dir().expect("couldn't get current working directory");
        args.paths = args.paths(cwd);
        args
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
                "can only be json, json_pretty, machine, or fdupes, found: {:?}",
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
                "can only be seahash, xxhash, or highway, found: {:?}",
                s
            )),
        }
    }
}
