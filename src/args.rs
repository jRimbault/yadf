use super::{Algorithm, Args, Format, ReplicationFactor};
use std::collections::HashSet;
use std::env;
use std::fmt;
use std::path::PathBuf;
use structopt::StructOpt;

impl Args {
    pub fn max(&self) -> Option<u64> {
        self.max.map(|m| m.get_bytes())
    }

    pub fn min(&self) -> Option<u64> {
        self.min
            .map(|m| m.get_bytes())
            .or(if self.no_empty { Some(1) } else { None })
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

    fn init_logger(&self) {
        env_logger::Builder::new()
            .filter_level(
                self.verbosity
                    .log_level()
                    .unwrap_or(log::Level::Error)
                    .to_level_filter(),
            )
            .init();
    }

    pub fn init_from_env() -> Self {
        let long_version = env!("YADF_BUILD_VERSION").replace("|", "\n");
        let app = Self::clap()
            .long_version(long_version.as_str())
            .after_help("For sizes, K/M/G/T[B|iB] suffixes can be used (case-insensitive).");
        let mut args = Self::from_clap(&app.get_matches());
        let cwd = || env::current_dir().expect("couldn't get current working directory");
        args.paths = args.paths(cwd);
        args.init_logger();
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

impl Default for ReplicationFactor {
    fn default() -> Self {
        ReplicationFactor::Over(1)
    }
}

impl std::str::FromStr for ReplicationFactor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        const SEPS: &[char] = &[':', '='];
        let mut arg = s.split(SEPS).map(|w| w.to_lowercase());
        let rf = match (arg.next().as_deref(), arg.next()) {
            (Some("under"), Some(value)) => {
                let factor = value.parse::<usize>().map_err(|e| e.to_string())?;
                ReplicationFactor::Under(factor)
            }
            (Some("equal"), Some(value)) => {
                let factor = value.parse::<usize>().map_err(|e| e.to_string())?;
                ReplicationFactor::Equal(factor)
            }
            (Some("over"), Some(value)) => {
                let factor = value.parse::<usize>().map_err(|e| e.to_string())?;
                ReplicationFactor::Over(factor)
            }
            _ => {
                return Err(format!(
                    "replication factor must be of the form \
                    `over:1` or `under:5` or `equal:2`, \
                    got {:?}",
                    s
                ))
            }
        };
        Ok(rf)
    }
}

impl fmt::Display for ReplicationFactor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl From<ReplicationFactor> for yadf::Factor {
    fn from(f: ReplicationFactor) -> Self {
        match f {
            ReplicationFactor::Under(n) => yadf::Factor::Under(n),
            ReplicationFactor::Equal(n) => yadf::Factor::Equal(n),
            ReplicationFactor::Over(n) => yadf::Factor::Over(n),
        }
    }
}
