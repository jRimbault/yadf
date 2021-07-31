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

pub trait WalkParallelForEach {
    fn for_each<F>(self, f: F)
    where
        F: Fn(Result<ignore::DirEntry, ignore::Error>) -> ignore::WalkState,
        F: Send + Copy;
}

impl WalkParallelForEach for ignore::WalkParallel {
    fn for_each<F>(self, f: F)
    where
        F: Fn(Result<ignore::DirEntry, ignore::Error>) -> ignore::WalkState,
        F: Send + Copy,
    {
        self.run(|| Box::new(move |result| f(result)))
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
