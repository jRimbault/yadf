use std::fs::Metadata;
use std::path::Path;

#[derive(Debug)]
pub struct FileFilter {
    min: Option<u64>,
    max: Option<u64>,
    regex: Option<regex::Regex>,
    glob: Option<globset::GlobMatcher>,
    inodes_filter: inode::Filter,
}

impl FileFilter {
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
        #[cfg(unix)]
        {
            if !self.inodes_filter.is_unique(&meta) {
                return false;
            }
        }
        meta.is_file()
            && self.min.map_or(true, |m| meta.len() >= m)
            && self.max.map_or(true, |m| meta.len() <= m)
            && is_match(&self.regex, path).unwrap_or(true)
            && is_match(&self.glob, path).unwrap_or(true)
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

mod inode {
    use std::collections::HashSet;
    use std::fs::Metadata;
    use std::sync::Mutex;

    /// Filter out unique inodes
    #[derive(Debug)]
    pub enum Filter {
        Disabled,
        Enabled(InodeSet),
    }

    #[derive(Debug, Default)]
    pub struct InodeSet(Mutex<HashSet<u64>>);

    impl Filter {
        pub fn new(disable_hard_links_filter: bool) -> Self {
            #[cfg(not(unix))]
            {
                return Self::Disabled;
            }
            if disable_hard_links_filter {
                Self::Disabled
            } else {
                Self::Enabled(Default::default())
            }
        }

        pub fn is_unique(&self, meta: &Metadata) -> bool {
            match self {
                Self::Disabled => true,
                Self::Enabled(set) => set.is_unique(meta),
            }
        }
    }

    impl InodeSet {
        #[cfg(unix)]
        fn is_unique(&self, meta: &Metadata) -> bool {
            use std::os::unix::fs::MetadataExt;
            self.0.lock().unwrap().insert(meta.ino())
        }
        #[cfg(not(unix))]
        fn is_unique(&self, meta: &Metadata) -> bool {
            true
        }
    }
}
