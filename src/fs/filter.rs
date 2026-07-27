use std::fs::Metadata;
use std::path::Path;

#[derive(Debug)]
pub struct FileFilter {
    min: Option<u64>,
    max: Option<u64>,
    regex: Option<regex::Regex>,
    glob: Option<globset::GlobMatcher>,
    #[cfg(unix)]
    inodes_filter: inode::Filter,
}

impl FileFilter {
    #[cfg(not(unix))]
    pub fn new(
        min: Option<u64>,
        max: Option<u64>,
        regex: Option<regex::Regex>,
        glob: Option<globset::GlobMatcher>,
    ) -> Self {
        Self {
            min,
            max,
            regex,
            glob,
        }
    }

    #[cfg(unix)]
    pub fn new(
        min: Option<u64>,
        max: Option<u64>,
        regex: Option<regex::Regex>,
        glob: Option<globset::GlobMatcher>,
        disable_hard_links_filter: bool,
    ) -> Self {
        Self {
            min,
            max,
            regex,
            glob,
            inodes_filter: inode::Filter::new(disable_hard_links_filter),
        }
    }

    pub fn is_match(&self, path: &Path, meta: Metadata) -> bool {
        // Cheap, lock-free predicates first: the inode check below takes a
        // shared lock, so files rejected by size/name never contend for it.
        let cheap = meta.is_file()
            && self.min.is_none_or(|m| meta.len() >= m)
            && self.max.is_none_or(|m| meta.len() <= m)
            && is_match(&self.regex, path).unwrap_or(true)
            && is_match(&self.glob, path).unwrap_or(true);
        if !cheap {
            return false;
        }
        #[cfg(unix)]
        {
            if !self.inodes_filter.is_unique(&meta) {
                return false;
            }
        }
        true
    }
}

fn is_match<M: Matcher>(opt: &Option<M>, path: &Path) -> Option<bool> {
    opt.as_ref().and_then(|m| m.is_file_name_match(path))
}

trait Matcher {
    fn is_file_name_match(&self, path: &Path) -> Option<bool>;
}

impl Matcher for regex::Regex {
    fn is_file_name_match(&self, path: &Path) -> Option<bool> {
        path.file_name()
            .and_then(std::ffi::OsStr::to_str)
            .map(|file_name| self.is_match(file_name))
    }
}

impl Matcher for globset::GlobMatcher {
    fn is_file_name_match(&self, path: &Path) -> Option<bool> {
        path.file_name().map(|file_name| self.is_match(file_name))
    }
}

#[cfg(unix)]
mod inode {
    use std::collections::HashSet;
    use std::fs::Metadata;
    use std::os::unix::fs::MetadataExt;
    use std::sync::Mutex;

    /// Inode numbers are only unique within a single device; two files on
    /// different filesystems can legitimately share an inode number.
    type Key = (u64, u64); // (dev, ino)

    /// Number of independently-locked buckets in [`InodeSet`]. Sharding
    /// keeps the hardlink check from becoming a hot, single-mutex
    /// bottleneck when many I/O threads hit it concurrently.
    const SHARD_COUNT: usize = 64;

    /// Filter out unique inodes
    #[derive(Debug)]
    pub enum Filter {
        Disabled,
        Enabled(Box<InodeSet>),
    }

    #[derive(Debug)]
    pub struct InodeSet {
        shards: [Mutex<HashSet<Key>>; SHARD_COUNT],
    }

    impl Default for InodeSet {
        fn default() -> Self {
            Self {
                shards: std::array::from_fn(|_| Mutex::default()),
            }
        }
    }

    impl Filter {
        pub fn new(disable_hard_links_filter: bool) -> Self {
            if disable_hard_links_filter {
                Self::Disabled
            } else {
                Self::Enabled(Default::default())
            }
        }

        pub fn is_unique(&self, meta: &Metadata) -> bool {
            match self {
                Self::Disabled => true,
                Self::Enabled(set) => set.is_unique(meta.dev(), meta.ino()),
            }
        }
    }

    impl InodeSet {
        fn is_unique(&self, dev: u64, ino: u64) -> bool {
            let shard = ino as usize % SHARD_COUNT;
            self.shards[shard].lock().unwrap().insert((dev, ino))
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn same_inode_number_on_different_devices_are_not_treated_as_hard_links() {
            let set = InodeSet::default();
            assert!(set.is_unique(1, 42), "first sighting of (dev=1, ino=42)");
            assert!(
                set.is_unique(2, 42),
                "same inode number on a different device is a distinct file"
            );
            assert!(
                !set.is_unique(1, 42),
                "same (dev, ino) seen again is a hard link"
            );
        }
    }
}
