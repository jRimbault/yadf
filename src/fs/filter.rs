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
    hard_links: bool,
    #[cfg(unix)]
    inodes_seen: Mutex<HashSet<u64>>,
}

impl FileFilter {
    pub fn new(
        min: Option<u64>,
        max: Option<u64>,
        regex: Option<regex::Regex>,
        glob: Option<globset::GlobMatcher>,
        hard_links: bool,
    ) -> Self {
        Self {
            min,
            max,
            regex,
            glob,
            hard_links,
            #[cfg(unix)]
            inodes_seen: Default::default(),
        }
    }

    pub fn is_match(&self, path: &Path, meta: Metadata) -> bool {
        #[cfg(unix)]
        {
            if !self.hard_links {
                let inode = meta.ino();
                if !self.inodes_seen.lock().unwrap().insert(inode) {
                    return false;
                }
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
