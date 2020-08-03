use serde::ser::{Serialize, Serializer};
use std::path::Path;

/// Serialization wrapper for [`ignore::DirEntry`](https://docs.rs/ignore/0.4.16/ignore/struct.DirEntry.html)
#[repr(transparent)]
pub struct DirEntry(pub ignore::DirEntry);

impl Serialize for DirEntry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Serialize::serialize(self.path(), serializer)
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

impl std::ops::Deref for DirEntry {
    type Target = ignore::DirEntry;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<Path> for DirEntry {
    fn as_ref(&self) -> &Path {
        self.path()
    }
}
