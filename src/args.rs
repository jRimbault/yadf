use super::{Algorithm, Args, Format};
use std::collections::HashSet;
use std::env;
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
