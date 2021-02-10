#[derive(Debug)]
#[repr(transparent)]
pub struct Path(std::path::PathBuf);

use serde::{Serialize, Serializer};

impl Serialize for Path {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.0.to_str() {
            Some(path) => serializer.serialize_str(path),
            None => {
                log::warn!("path contains invalid UTF-8 characters : {:?}", self.0);
                serializer.serialize_str(&grep_cli::escape_os(self.0.as_ref()))
            }
        }
    }
}

impl<T> From<T> for Path
where
    T: Into<std::path::PathBuf>,
{
    fn from(path: T) -> Self {
        Self(path.into())
    }
}

impl AsRef<std::path::Path> for Path {
    fn as_ref(&self) -> &std::path::Path {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    #[test]
    fn shouldnt_panic_on_invalid_utf8_path() {
        use super::*;
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        use std::path::PathBuf;
        // asserts its invalidity
        let invalid_utf8: &[u8] = b"\xe7\xe7";
        assert!(String::from_utf8(invalid_utf8.to_vec()).is_err());
        // without wrapper it errors
        let path = PathBuf::from(OsString::from_vec(invalid_utf8.to_vec()));
        assert!(serde_json::to_string(&path).is_err());
        // with wrapper it's ok
        let path = Path(PathBuf::from(OsString::from_vec(invalid_utf8.to_vec())));
        assert!(serde_json::to_string(&path).is_ok());
    }
}
