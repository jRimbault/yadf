use std::collections::HashSet;
use std::hash::Hash;
use std::path::Path;

/// Could be replaced by `unique_by` in `itertools`
pub trait IteratorExt: Iterator + Sized {
    fn unique_by<F, K>(self, f: F) -> UniqueBy<Self, F, K>
    where
        F: Fn(&Self::Item) -> K,
        K: Hash + Eq,
    {
        UniqueBy::new(self, f)
    }
}

impl<I: Iterator> IteratorExt for I {}

pub struct UniqueBy<I, F, K> {
    iter: I,
    set: HashSet<K>,
    f: F,
}

impl<I, F, K> UniqueBy<I, F, K>
where
    I: Iterator,
    F: Fn(&I::Item) -> K,
    K: Hash + Eq,
{
    fn new(iter: I, f: F) -> Self {
        Self {
            iter,
            f,
            set: HashSet::new(),
        }
    }
}

#[allow(clippy::manual_find)]
impl<I, F, K> Iterator for UniqueBy<I, F, K>
where
    I: Iterator,
    F: Fn(&I::Item) -> K,
    K: Hash + Eq,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        for item in &mut self.iter {
            if self.set.insert((self.f)(&item)) {
                return Some(item);
            }
        }
        None
    }
}

pub trait WalkBuilderAddPaths {
    fn add_paths<P, I>(&mut self, paths: I) -> &mut Self
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>;
}

impl WalkBuilderAddPaths for ignore::WalkBuilder {
    fn add_paths<P, I>(&mut self, paths: I) -> &mut Self
    where
        P: AsRef<Path>,
        I: IntoIterator<Item = P>,
    {
        for path in paths {
            self.add(path);
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_by_filters_duplicates_and_exhausts() {
        let mut iter = [1, 2, 1, 3, 2].into_iter().unique_by(|&x| x);
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn add_paths_adds_every_path() {
        let mut builder = ignore::WalkBuilder::new(".");
        builder.add_paths(["src", "Cargo.toml"]);
        let count = builder.build().count();
        assert!(count > 1);
    }
}
