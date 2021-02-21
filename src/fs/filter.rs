use std::fs::Metadata;
use std::path::Path;

#[cfg(unix)]
use std::{collections::HashSet, os::unix::fs::MetadataExt, sync::Mutex};

#[derive(Debug)]
pub(crate) struct FileFilter {
    min: Option<u64>,
    max: Option<u64>,
    regex: Option<regex::Regex>,
    glob: Option<globset::GlobMatcher>,
    #[cfg(unix)]
    inodes_filter: InodeFilter,
}

#[derive(Debug)]
#[cfg(unix)]
struct InodeFilter {
    enabled: bool,
    seen: Mutex<HashSet<u64>>,
}

impl FileFilter {
    pub fn new(
        min: Option<u64>,
        max: Option<u64>,
        regex: Option<regex::Regex>,
        glob: Option<globset::GlobMatcher>,
        #[cfg(unix)] disable_hard_links_filter: bool,
    ) -> Self {
        Self {
            min,
            max,
            regex,
            glob,
            #[cfg(unix)]
            inodes_filter: InodeFilter {
                enabled: !disable_hard_links_filter,
                seen: Default::default(),
            },
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

#[cfg(unix)]
impl InodeFilter {
    fn is_unique(&self, meta: &Metadata) -> bool {
        if !self.enabled {
            return true;
        }
        self.seen.lock().unwrap().insert(meta.ino())
    }
}

fn is_match<M: Matcher>(opt: &Option<M>, path: &Path) -> Option<bool> {
    opt.as_ref().and_then(|m| m.is_file_name_match(path))
}

trait Matcher {
    fn is_file_name_match(&self, path: &Path) -> Option<bool>;
}

impl Matcher for regex::Regex {
    #[inline(always)]
    fn is_file_name_match(&self, path: &Path) -> Option<bool> {
        path.file_name()
            .and_then(|p| p.to_str())
            .map(|file_name| self.is_match(file_name))
    }
}

impl Matcher for globset::GlobMatcher {
    #[inline(always)]
    fn is_file_name_match(&self, path: &Path) -> Option<bool> {
        path.file_name().map(|file_name| self.is_match(file_name))
    }
}
