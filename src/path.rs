//! Memory-efficient path representation.
//!
//! A [`std::path::PathBuf`] owns its whole text, so a million files under the
//! same directory keep a million copies of that directory's name.
//! [`Path`] stores only the last component of a path plus a
//! reference-counted pointer to its parent, so siblings share their common
//! ancestry. The design follows the `Path` type of `fclones`.
//!
//! # Differences from the `fclones` type
//!
//! - The component is a `Box<OsStr>` rather than a `CString`. It has the same
//!   size (a fat pointer), is safe to build from any `OsStr` on every platform
//!   (no interior-NUL failure mode, no `unsafe` needed to get the `OsStr`
//!   back, which this crate forbids), and needs no trailing NUL byte.
//! - Paths are decomposed with [`std::path::Path::components`], so, exactly
//!   like `PathBuf` equality, a trailing slash, repeated slashes and inner `.`
//!   components are not preserved. `..` and a leading `.` are kept. The root
//!   (and, on Windows, the prefix) is stored as a component of its own.
//! - `Ord` is *not* derived: a derived order would compare the parent chain
//!   first, which does not match std. Here ordering is component by component
//!   from the root and agrees exactly with `PathBuf`'s. `Eq` also agrees with
//!   `PathBuf`.
//! - `Hash` is consistent with `Eq` but does not produce the same hash value as
//!   `PathBuf` would.

use std::cell::Cell;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::{Component, Path as StdPath, PathBuf};
use std::sync::{Arc, Mutex};

/// Shares its parent directories with the other paths of a scan, instead of
/// owning a copy of their text like a [`std::path::PathBuf`].
#[derive(Clone)]
pub struct Path {
    parent: Option<Arc<Path>>,
    /// One path component as returned by `Component::as_os_str`. Empty only
    /// for the empty path.
    component: Box<OsStr>,
}

impl Path {
    /// Builds a path, with no parent shared with anything else.
    pub fn from_path(path: &StdPath) -> Self {
        Self::chain(None, path).unwrap_or_else(|| Self {
            parent: None,
            component: Box::from(OsStr::new("")),
        })
    }

    /// Builds the path `parent/relative`, sharing `parent` instead of
    /// copying it. This is what a directory walker should use.
    pub fn with_parent(parent: &Arc<Self>, relative: &StdPath) -> Self {
        Self::chain(Some(Arc::clone(parent)), relative).unwrap_or_else(|| Self::clone(parent))
    }

    /// Appends `name` to `self`; shorthand for [`Path::with_parent`].
    pub fn join(self: &Arc<Self>, name: &OsStr) -> Self {
        Self::with_parent(self, StdPath::new(name))
    }

    /// Hangs the components of `path` under `parent`, one node each, and
    /// returns the leaf. `None` when `path` has no components.
    fn chain(parent: Option<Arc<Self>>, path: &StdPath) -> Option<Self> {
        let mut components = path.components();
        let first = Self {
            parent,
            component: Box::from(components.next()?.as_os_str()),
        };
        Some(components.fold(first, |node, component| Self {
            parent: Some(Arc::new(node)),
            component: Box::from(component.as_os_str()),
        }))
    }

    /// The shared parent, if any.
    pub fn parent(&self) -> Option<&Arc<Self>> {
        self.parent.as_ref()
    }

    /// The last component of the path.
    pub fn file_name(&self) -> &OsStr {
        &self.component
    }

    /// Number of non-empty components.
    pub fn depth(&self) -> usize {
        self.ancestors()
            .filter(|node| !node.component.is_empty())
            .count()
    }

    /// Whether `self` has the same components as `path`, without allocating.
    pub fn eq_path(&self, path: &StdPath) -> bool {
        let mut theirs = path.components().rev();
        for node in self.ancestors().filter(|node| !node.component.is_empty()) {
            match theirs.next() {
                Some(component) if component.as_os_str() == &*node.component => {}
                _ => return false,
            }
        }
        theirs.next().is_none()
    }

    /// Reconstructs the standard path.
    pub fn to_path_buf(&self) -> PathBuf {
        let mut path = PathBuf::new();
        self.write_to(&mut path);
        path
    }

    /// Overwrites `buf` with the standard path, reusing its allocation.
    pub fn write_to(&self, buf: &mut PathBuf) {
        buf.clear();
        self.push_onto(buf);
    }

    /// Calls `f` with the standard path, built in a per-thread buffer, so
    /// opening a file does not allocate a fresh `PathBuf` each time.
    pub fn with_std_path<R>(&self, f: impl FnOnce(&StdPath) -> R) -> R {
        thread_local! {
            static SCRATCH: Cell<PathBuf> = Cell::new(PathBuf::new());
        }
        SCRATCH.with(|scratch| {
            // Taken rather than borrowed: a nested call from `f` gets a fresh
            // buffer instead of a borrow conflict.
            let mut buf = scratch.take();
            self.write_to(&mut buf);
            let result = f(&buf);
            scratch.set(buf);
            result
        })
    }

    /// Root-first push of the non-empty components. Recursion depth is the
    /// component count, bounded in practice by `PATH_MAX`.
    fn push_onto(&self, buf: &mut PathBuf) {
        if let Some(parent) = &self.parent {
            parent.push_onto(buf);
        }
        if !self.component.is_empty() {
            buf.push(&*self.component);
        }
    }

    /// Leaf-to-root iterator over this node and its ancestors.
    fn ancestors(&self) -> impl Iterator<Item = &Self> {
        std::iter::successors(Some(self), |node| node.parent.as_deref())
    }

    /// Non-empty components, root first.
    fn parts(&self) -> Vec<&OsStr> {
        let mut parts: Vec<&OsStr> = self
            .ancestors()
            .map(|node| &*node.component)
            .filter(|part| !part.is_empty())
            .collect();
        parts.reverse();
        parts
    }

    /// The component as std classifies it, which gives std's ordering.
    fn as_component(part: &OsStr) -> Component<'_> {
        StdPath::new(part)
            .components()
            .next()
            .expect("stored components are never empty")
    }
}

/// Shares directory nodes between the paths of a whole directory walk.
///
/// A directory walker visits many directories, and building each path with
/// [`Path::from_path`] would copy every ancestor again for each new
/// directory. The interner hands out one [`Arc`] per directory instead, so
/// `/a/b` and `/a/c` share their `/a` as well as their siblings. Drop it once
/// the walk is over: it holds a full `PathBuf` per directory.
#[derive(Debug, Default)]
pub struct Interner {
    dirs: Mutex<HashMap<PathBuf, Arc<Path>>>,
}

impl Interner {
    /// The shared node for the directory `dir`.
    pub fn dir(&self, dir: &StdPath) -> Arc<Path> {
        let mut dirs = self
            .dirs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Self::dir_in(&mut dirs, dir)
    }

    fn dir_in(dirs: &mut HashMap<PathBuf, Arc<Path>>, dir: &StdPath) -> Arc<Path> {
        if let Some(node) = dirs.get(dir) {
            return Arc::clone(node);
        }
        let node = match dir.parent() {
            Some(parent) if !parent.as_os_str().is_empty() => {
                let parent = Self::dir_in(dirs, parent);
                match dir.components().next_back() {
                    Some(last) => Arc::new(Path {
                        parent: Some(parent),
                        component: Box::from(last.as_os_str()),
                    }),
                    None => parent,
                }
            }
            _ => Arc::new(Path::from_path(dir)),
        };
        dirs.insert(dir.to_path_buf(), Arc::clone(&node));
        node
    }

    /// Builds `path` under its interned parent directory. `last` is a
    /// per-thread cache of the previous parent: paths arriving grouped by
    /// directory then skip the lock and the lookup entirely.
    pub fn path(&self, path: &StdPath, last: &mut LastDir) -> Path {
        let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
            return Path::from_path(path);
        };
        if parent.as_os_str().is_empty() {
            return Path::from_path(path);
        }
        if let Some(cached) = last.get(parent) {
            return cached.join(name);
        }
        let dir = self.dir(parent);
        let child = dir.join(name);
        last.set(parent, dir);
        child
    }
}

/// The directory a walker thread interned last, keyed by its exact text.
///
/// A walker hands out every entry of a directory with the same parent
/// spelling, so a byte comparison is enough to recognize it. That is cheaper
/// than a component-wise [`Path::eq_path`], and it runs once per file.
#[derive(Debug, Default)]
pub struct LastDir {
    text: OsString,
    node: Option<Arc<Path>>,
}

impl LastDir {
    fn get(&self, dir: &StdPath) -> Option<&Arc<Path>> {
        self.node
            .as_ref()
            .filter(|_| self.text.as_os_str() == dir.as_os_str())
    }

    fn set(&mut self, dir: &StdPath, node: Arc<Path>) {
        // Reuses the text allocation across directories.
        self.text.clear();
        self.text.push(dir.as_os_str());
        self.node = Some(node);
    }
}

/// Unlinks the chain iteratively: the derived drop would recurse once per
/// ancestor that this path is the last owner of.
impl Drop for Path {
    fn drop(&mut self) {
        let mut parent = self.parent.take();
        while let Some(node) = parent {
            // `into_inner` hands the node to exactly one of the concurrent
            // last owners; everyone else just decrements.
            parent = Arc::into_inner(node).and_then(|mut node| node.parent.take());
        }
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        let (mut a, mut b) = (Some(self), Some(other));
        loop {
            match (a, b) {
                (None, None) => return true,
                (Some(x), Some(y)) => {
                    if std::ptr::eq(x, y) {
                        return true;
                    }
                    if x.component != y.component {
                        return false;
                    }
                    a = x.parent.as_deref();
                    b = y.parent.as_deref();
                }
                _ => return false,
            }
        }
    }
}

impl Eq for Path {}

impl PartialEq<StdPath> for Path {
    fn eq(&self, other: &StdPath) -> bool {
        self.eq_path(other)
    }
}

impl Hash for Path {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Root-first, like the equality it must agree with.
        if let Some(parent) = &self.parent {
            parent.hash(state);
        }
        if !self.component.is_empty() {
            self.component.hash(state);
        }
    }
}

impl PartialOrd for Path {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Path {
    fn cmp(&self, other: &Self) -> Ordering {
        if std::ptr::eq(self, other) {
            return Ordering::Equal;
        }
        self.parts()
            .into_iter()
            .map(Self::as_component)
            .cmp(other.parts().into_iter().map(Self::as_component))
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with_std_path(|path| path.display().fmt(f))
    }
}

impl fmt::Debug for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.with_std_path(|path| fmt::Debug::fmt(path, f))
    }
}

/// Any owned or borrowed standard path converts, as long as it can become a
/// [`PathBuf`].
impl<T> From<T> for Path
where
    T: Into<PathBuf>,
{
    fn from(path: T) -> Self {
        Self::from_path(&path.into())
    }
}

impl serde::Serialize for Path {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.collect_str(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;
    use std::mem::size_of;

    fn hash_of<T: Hash>(value: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    const SAMPLES: &[&str] = &[
        "",
        "/",
        "/a",
        "/a/b/c.txt",
        "a",
        "a/b",
        "a/b/c",
        "a/b/",
        "a//b",
        "a/./b",
        "./a",
        "./a/b",
        "../a",
        "a/../b",
        "..",
        ".",
        "-",
        "a-b",
        "a b/c d",
        "/home/user/Documents/file.tar.gz",
    ];

    #[test]
    fn round_trips_equal_to_pathbuf() {
        for sample in SAMPLES {
            let original = PathBuf::from(sample);
            let compact = Path::from_path(&original);
            assert_eq!(compact.to_path_buf(), original, "{sample:?}");
            assert!(compact.eq_path(&original), "{sample:?}");
        }
    }

    #[test]
    fn display_and_debug_match_pathbuf() {
        // Components are rebuilt, so on Windows the separators come out
        // normalized where `PathBuf` would keep the `/` it was given.
        let original: PathBuf = StdPath::new("/a/b c/d.txt").components().collect();
        let compact = Path::from(&original);
        assert_eq!(compact.to_string(), original.display().to_string());
        assert_eq!(format!("{compact:?}"), format!("{original:?}"));
    }

    #[test]
    fn root_and_empty() {
        let root = Path::from_path(StdPath::new("/"));
        assert!(root.parent().is_none());
        assert_eq!(root.depth(), 1);
        assert_eq!(root.to_path_buf(), PathBuf::from("/"));
        let empty = Path::from_path(StdPath::new(""));
        assert_eq!(empty.depth(), 0);
        assert_eq!(empty.to_path_buf(), PathBuf::new());
        assert_ne!(empty, root);
    }

    #[test]
    fn normalizes_like_pathbuf() {
        let a = Path::from_path(StdPath::new("a//b/./c/"));
        let b = Path::from_path(StdPath::new("a/b/c"));
        assert_eq!(a, b);
        assert_eq!(hash_of(&a), hash_of(&b));
        assert_ne!(
            Path::from_path(StdPath::new("a/../b")),
            Path::from_path(StdPath::new("b"))
        );
    }

    #[test]
    fn long_paths() {
        let long: PathBuf = (0..500).map(|i| format!("dir{i}")).collect();
        let compact = Path::from_path(&long);
        assert_eq!(compact.depth(), 500);
        assert_eq!(compact.to_path_buf(), long);
        assert_eq!(compact, Path::from_path(&long));
    }

    #[cfg(unix)]
    #[test]
    fn non_utf8() {
        use std::ffi::OsString;
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let bytes = b"/tmp/\xe7\xe7/\xff.bin".to_vec();
        let original = PathBuf::from(OsString::from_vec(bytes.clone()));
        let compact = Path::from_path(&original);
        assert_eq!(compact.to_path_buf().as_os_str().as_bytes(), &bytes[..]);
        assert_eq!(compact.file_name().as_bytes(), b"\xff.bin");
        assert!(!compact.to_string().is_empty());
    }

    #[test]
    fn parents_are_shared() {
        let dir = Arc::new(Path::from_path(StdPath::new("/some/dir")));
        let a = dir.join(OsStr::new("a"));
        let b = dir.join(OsStr::new("b"));
        assert!(Arc::ptr_eq(a.parent().unwrap(), &dir));
        assert!(Arc::ptr_eq(a.parent().unwrap(), b.parent().unwrap()));
        assert_eq!(Arc::strong_count(&dir), 3);
        drop(a);
        assert_eq!(Arc::strong_count(&dir), 2);
        assert_eq!(b.to_path_buf(), PathBuf::from("/some/dir/b"));
    }

    #[test]
    fn with_parent_multiple_components() {
        let dir = Arc::new(Path::from_path(StdPath::new("/x")));
        let child = Path::with_parent(&dir, StdPath::new("y/z"));
        assert_eq!(child.to_path_buf(), PathBuf::from("/x/y/z"));
        let same = Path::with_parent(&dir, StdPath::new(""));
        assert_eq!(same, *dir);
    }

    #[test]
    fn write_to_and_with_std_path_match_to_path_buf() {
        let mut buf = PathBuf::from("/stale/content/that/is/longer");
        for sample in SAMPLES {
            let compact = Path::from_path(StdPath::new(sample));
            compact.write_to(&mut buf);
            assert_eq!(buf, compact.to_path_buf(), "{sample:?}");
            compact.with_std_path(|path| assert_eq!(path, buf, "{sample:?}"));
        }
        // A nested call gets its own buffer.
        let (a, b) = (Path::from("/a"), Path::from("/b"));
        a.with_std_path(|outer| {
            b.with_std_path(|inner| assert_eq!(inner, StdPath::new("/b")));
            assert_eq!(outer, StdPath::new("/a"));
        });
    }

    #[test]
    fn deep_chain_drops_without_recursion() {
        let deep: PathBuf = (0..200_000).map(|_| "d").collect();
        let path = Path::from_path(&deep);
        assert_eq!(path.depth(), 200_000);
        drop(path);
    }

    #[test]
    fn last_dir_matches_exact_text_only() {
        let interner = Interner::default();
        let mut last = LastDir::default();
        let a = interner.path(StdPath::new("/p/q/a"), &mut last);
        let b = interner.path(StdPath::new("/p/q/b"), &mut last);
        assert!(Arc::ptr_eq(a.parent().unwrap(), b.parent().unwrap()));
        // Same directory, different spelling: misses the cache but the
        // interner still resolves it to an equal path.
        let c = interner.path(StdPath::new("/p/q//c"), &mut last);
        assert_eq!(c.to_path_buf(), PathBuf::from("/p/q/c"));
        let d = interner.path(StdPath::new("/p/r/d"), &mut last);
        assert!(!Arc::ptr_eq(a.parent().unwrap(), d.parent().unwrap()));
        assert_eq!(d.to_path_buf(), PathBuf::from("/p/r/d"));
    }

    #[test]
    fn eq_and_hash_agree_across_construction() {
        let built = Path::from_path(StdPath::new("/a/b/c"));
        let dir = Arc::new(Path::from_path(StdPath::new("/a/b")));
        let joined = dir.join(OsStr::new("c"));
        assert_eq!(built, joined);
        assert_eq!(hash_of(&built), hash_of(&joined));
        assert_ne!(built, *dir);
    }

    #[test]
    fn ordering_matches_pathbuf() {
        for a in SAMPLES {
            for b in SAMPLES {
                let (pa, pb) = (PathBuf::from(a), PathBuf::from(b));
                let (ca, cb) = (Path::from_path(&pa), Path::from_path(&pb));
                assert_eq!(ca.cmp(&cb), pa.cmp(&pb), "{a:?} vs {b:?}");
                assert_eq!(ca == cb, pa == pb, "{a:?} vs {b:?}");
                if ca == cb {
                    assert_eq!(hash_of(&ca), hash_of(&cb), "{a:?} vs {b:?}");
                }
            }
        }
    }

    #[test]
    fn sorting_matches_pathbuf() {
        let mut std_paths: Vec<PathBuf> = SAMPLES.iter().map(PathBuf::from).collect();
        let mut compact: Vec<Path> = std_paths.iter().map(Path::from).collect();
        std_paths.sort();
        compact.sort();
        let back: Vec<PathBuf> = compact.iter().map(Path::to_path_buf).collect();
        assert_eq!(back, std_paths);
    }

    #[test]
    fn component_is_not_larger_than_cstring() {
        assert_eq!(size_of::<Box<OsStr>>(), size_of::<std::ffi::CString>());
        assert_eq!(size_of::<Path>(), 24);
    }

    #[test]
    fn sharing_saves_memory_for_siblings() {
        let parent: PathBuf = (0..8).map(|i| format!("some-long-directory-{i}")).collect();
        let parent = PathBuf::from("/").join(parent);
        let names: Vec<String> = (0..1000).map(|i| format!("file-{i:05}.dat")).collect();

        let std_paths: Vec<PathBuf> = names.iter().map(|name| parent.join(name)).collect();
        // Struct + heap text.
        let std_bytes: usize = std_paths
            .iter()
            .map(|path| size_of::<PathBuf>() + path.as_os_str().len())
            .sum();

        let dir = Arc::new(Path::from_path(&parent));
        let compact: Vec<Path> = names
            .iter()
            .map(|name| dir.join(OsStr::new(name)))
            .collect();
        assert_eq!(Arc::strong_count(&dir), 1001);
        // Each leaf: struct + its own name. The directory chain is paid once
        // (node + Arc counters + name per component).
        let node = size_of::<Path>() + 2 * size_of::<usize>();
        let chain: usize = dir.ancestors().map(|n| node + n.component.len()).sum();
        let compact_bytes: usize = chain
            + compact
                .iter()
                .map(|path| size_of::<Path>() + path.file_name().len())
                .sum::<usize>();

        assert!(
            compact_bytes * 2 < std_bytes,
            "compact {compact_bytes} B vs std {std_bytes} B"
        );
        for (c, s) in compact.iter().zip(&std_paths) {
            assert!(c.eq_path(s));
        }
    }

    #[test]
    fn interner_shares_ancestors_across_directories() {
        let interner = Interner::default();
        let mut last = LastDir::default();
        let a = interner.path(StdPath::new("/root/a/one"), &mut last);
        let a2 = interner.path(StdPath::new("/root/a/two"), &mut last);
        let b = interner.path(StdPath::new("/root/b/one"), &mut last);
        assert!(Arc::ptr_eq(a.parent().unwrap(), a2.parent().unwrap()));
        let root_of = |p: &Path| Arc::clone(p.parent().unwrap().parent().unwrap());
        assert!(Arc::ptr_eq(&root_of(&a), &root_of(&b)));
        assert_eq!(a.to_path_buf(), StdPath::new("/root/a/one"));
        assert_eq!(b.to_path_buf(), StdPath::new("/root/b/one"));
        // Relative and top-level paths still round-trip.
        for raw in ["rel/x", "x", "/x", "./rel/y"] {
            let path = interner.path(StdPath::new(raw), &mut last);
            assert_eq!(path.to_path_buf(), StdPath::new(raw));
        }
    }
}
