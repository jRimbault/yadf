use super::Args;
use std::collections::HashSet;
use std::env;
use std::fmt;
use std::path::PathBuf;
use structopt::StructOpt;

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
    pub fn file_constraints(&self) -> (Option<u64>, Option<u64>) {
        (
            self.min
                .map(|m| m.get_bytes())
                .or(if self.no_empty { Some(1) } else { None }),
            self.max.map(|m| m.get_bytes()),
        )
    }

    /// returns a list of the deduped paths
    fn paths(&self, cwd: PathBuf) -> Vec<PathBuf> {
        if self.paths.is_empty() {
            vec![cwd]
        } else {
            self.paths
                .iter()
                .cloned()
                .collect::<HashSet<PathBuf>>()
                .into_iter()
                .collect()
        }
    }

    pub fn from_env() -> Self {
        let mut args = Self::from_args();
        let cwd = env::current_dir().expect("couldn't get current working directory");
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

#[cfg(test)]
mod tests {
    mod constraints {
        use super::super::*;

        #[test]
        fn default() {
            let args = Args {
                no_empty: false,
                min: None,
                max: None,
                ..Args::default()
            };
            let constraints = args.file_constraints();
            assert_eq!(constraints, (None, None));
        }

        #[test]
        fn no_empty() {
            let args = Args {
                no_empty: true,
                min: None,
                max: None,
                ..Args::default()
            };
            let constraints = args.file_constraints();
            assert_eq!(constraints, (Some(1), None));
        }

        #[test]
        fn min_one_kibibyte_and_half() {
            let args = Args {
                no_empty: false,
                min: Some("1.5kib".parse().unwrap()),
                max: None,
                ..Args::default()
            };
            let constraints = args.file_constraints();
            assert_eq!(constraints, (Some(1536), None));
        }

        #[test]
        fn max_block_size() {
            let args = Args {
                no_empty: true,
                min: None,
                max: Some("4ki".parse().unwrap()),
                ..Args::default()
            };
            let constraints = args.file_constraints();
            assert_eq!(constraints, (Some(1), Some(4096)));
        }
    }

    mod paths {
        use super::super::*;

        #[test]
        fn if_empty_should_get_cwd() {
            let args = Args::default();
            let cwd = PathBuf::from("default");
            let paths = args.paths(cwd.clone());
            assert_eq!(paths[0], cwd);
        }

        #[test]
        fn should_remove_duplicates() {
            let args = Args {
                paths: ["hello", "world", "hello", "world"]
                    .iter()
                    .map(PathBuf::from)
                    .collect(),
                ..Default::default()
            };
            let paths = args.paths("cwd".into());
            assert_eq!(args.paths.len(), 4);
            assert_eq!(paths.len(), 2);
        }
    }
}
