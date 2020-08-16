use std::fmt;

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
