use serde::ser::{Serialize, Serializer};
use std::path::Path;

/// Serialization wrapper for [`ignore::DirEntry`](https://docs.rs/ignore/0.4.16/ignore/struct.DirEntry.html)
#[repr(transparent)]
pub struct DirEntry(pub ignore::DirEntry);

impl DirEntry {
    pub fn path(&self) -> &Path {
        self.0.path()
    }

    pub fn metadata(&self) -> Result<std::fs::Metadata, ignore::Error> {
        self.0.metadata()
    }
}

impl Serialize for DirEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Serialize::serialize(self.0.path(), serializer)
    }
}

impl std::fmt::Display for DirEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.path().display().fmt(f)
    }
}

impl std::fmt::Debug for DirEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.path().display().fmt(f)
    }
}
