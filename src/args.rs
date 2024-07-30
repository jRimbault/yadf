use super::{Args, ReplicationFactor};
use clap::{CommandFactory, FromArgMatches};
use std::env;
use std::fmt;
use std::io::BufRead;
use std::path::PathBuf;

impl Args {
    pub fn max(&self) -> Option<u64> {
        self.max
            .as_ref()
            .map(|m| m.0.get_adjusted_unit(byte_unit::Unit::B))
            .map(|u| u.get_value() as _)
    }

    pub fn min(&self) -> Option<u64> {
        self.min
            .as_ref()
            .map(|m| m.0.get_adjusted_unit(byte_unit::Unit::B))
            .map(|u| u.get_value() as _)
            .or(if self.no_empty { Some(1) } else { None })
    }

    pub fn init_from_env() -> Self {
        let long_version = env!("YADF_BUILD_VERSION").replace('|', "\n");
        let short_version = long_version.lines().next().unwrap().to_string();
        let app = Self::command()
            .version(short_version)
            .long_version(long_version)
            .after_help("For sizes, K/M/G/T[B|iB] suffixes can be used (case-insensitive).");
        let mut args = Self::from_arg_matches(&app.get_matches()).unwrap();
        init_logger(&args.verbosity);
        args.build_paths();
        args
    }

    fn build_paths(&mut self) {
        if self.paths.is_empty() {
            self.paths = default_paths()
        }
    }
}

fn init_logger(verbosity: &clap_verbosity_flag::Verbosity) {
    env_logger::Builder::new()
        .filter_level(
            verbosity
                .log_level()
                .unwrap_or(log::Level::Error)
                .to_level_filter(),
        )
        .init();
}

fn default_paths() -> Vec<PathBuf> {
    let stdin = std::io::stdin();
    let mut paths = if std::io::IsTerminal::is_terminal(&stdin) {
        Vec::new()
    } else {
        stdin
            .lock()
            .lines()
            .map_while(Result::ok)
            .map(Into::into)
            .collect()
    };
    if paths.is_empty() {
        paths.push(env::current_dir().expect("couldn't get current working directory"));
    }
    paths
}

impl Default for ReplicationFactor {
    fn default() -> Self {
        ReplicationFactor::Over(1)
    }
}

impl std::str::FromStr for ReplicationFactor {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        use ReplicationFactor::*;
        const SEPS: &[char] = &[':', '='];
        let mut arg = value.split(SEPS);

        let rf = match (
            arg.next().map(str::to_lowercase).as_deref(),
            arg.next().and_then(|v| v.parse().ok()),
        ) {
            (Some("under"), Some(factor)) => Under(factor),
            (Some("equal"), Some(factor)) => Equal(factor),
            (Some("over"), Some(factor)) => Over(factor),
            _ => {
                return Err(format!(
                    "replication factor must be of the form \
                    `over:1` or `under:5` or `equal:2`, \
                    got {:?}",
                    value
                ))
            }
        };
        Ok(rf)
    }
}

impl fmt::Display for ReplicationFactor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replication_factor_parsing() {
        let cases = [
            ("under=6", ReplicationFactor::Under(6)),
            ("over:7", ReplicationFactor::Over(7)),
            ("over:1", ReplicationFactor::Over(1)),
            ("equal=3", ReplicationFactor::Equal(3)),
        ];

        for (value, expected) in cases.iter() {
            let rf: ReplicationFactor = value.parse().unwrap();
            assert_eq!(&rf, expected);
        }
    }
}
