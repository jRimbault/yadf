mod display;
mod serialize;

use std::collections::BTreeMap;
use std::iter::FromIterator;

/// Counter structure.
///
/// # Example :
///
/// ```
/// use yadf::TreeBag;
///
/// let bag: TreeBag<i32, &str> = vec![
///     (3, "hello world"),
///     (3, "foobar"),
///     (7, "fizz"),
///     (7, "buzz"),
///     (6, "rust"),
/// ].into_iter().collect();
/// assert_eq!(bag.as_inner()[&3].len(), 2);
/// assert_eq!(bag.as_inner()[&6].len(), 1);
/// assert_eq!(bag.as_inner()[&3][0], "hello world");
/// ```
#[repr(transparent)]
#[derive(Debug)]
pub struct TreeBag<H, T>(pub(crate) BTreeMap<H, Vec<T>>);

#[derive(Debug)]
pub enum Factor {
    Under(usize),
    Equal(usize),
    Over(usize),
}

/// A view which only provides access to n replicated entries
#[derive(Debug)]
pub struct Replicates<'a, H, T> {
    tree: &'a TreeBag<H, T>,
    factor: Factor,
}

#[derive(Debug)]
pub struct ReplicatesIter<'a, H, T> {
    iterator: std::collections::btree_map::Values<'a, H, Vec<T>>,
    factor: &'a Factor,
}

/// Display marker.
#[derive(Debug)]
pub struct Fdupes;
/// Display marker.
#[derive(Debug)]
pub struct Machine;

#[derive(Debug)]
pub struct Display<'a, H, T, U> {
    _marker: std::marker::PhantomData<&'a U>,
    tree: &'a Replicates<'a, H, T>,
}

impl<H, T> TreeBag<H, T> {
    /// Provides a view only on the buckets containing more than one element.
    pub const fn duplicates(&self) -> Replicates<'_, H, T> {
        Replicates {
            tree: self,
            factor: Factor::Over(1),
        }
    }

    pub const fn replicates(&self, factor: Factor) -> Replicates<'_, H, T> {
        Replicates { tree: self, factor }
    }

    /// Borrows the backing `BTreeMap` of the bag
    pub const fn as_inner(&self) -> &BTreeMap<H, Vec<T>> {
        &self.0
    }

    /// Mutably borrows the backing `BTreeMap` of the bag
    pub fn as_inner_mut(&mut self) -> &mut BTreeMap<H, Vec<T>> {
        &mut self.0
    }

    pub fn into_inner(self) -> BTreeMap<H, Vec<T>> {
        self.0
    }
}

impl<H, T> Replicates<'_, H, T> {
    /// Iterator over the buckets
    pub fn iter(&self) -> ReplicatesIter<'_, H, T> {
        ReplicatesIter {
            iterator: self.tree.0.values(),
            factor: &self.factor,
        }
    }

    /// Returns an object that implements [`Display`](std::fmt::Display)
    ///
    /// Depending on the contents of the [`TreeBag`](TreeBag), the display object
    /// can be parameterized to get a different `Display` implemenation.
    pub fn display<D>(&self) -> Display<'_, H, T, D> {
        Display {
            _marker: std::marker::PhantomData,
            tree: self,
        }
    }
}

impl<H: Ord, T> FromIterator<(H, T)> for TreeBag<H, T> {
    fn from_iter<I: IntoIterator<Item = (H, T)>>(iter: I) -> Self {
        let mut map: BTreeMap<H, Vec<T>> = Default::default();
        for (hash, item) in iter {
            map.entry(hash).or_default().push(item);
        }
        Self(map)
    }
}

impl<H, T> AsRef<BTreeMap<H, Vec<T>>> for TreeBag<H, T> {
    fn as_ref(&self) -> &BTreeMap<H, Vec<T>> {
        self.as_inner()
    }
}

impl<H, T> From<BTreeMap<H, Vec<T>>> for TreeBag<H, T> {
    fn from(value: BTreeMap<H, Vec<T>>) -> Self {
        Self(value)
    }
}

impl<'a, H, T> Iterator for ReplicatesIter<'a, H, T> {
    type Item = &'a [T];

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(bucket) = self.iterator.next() {
            if self.factor.pass(bucket.len()) {
                return Some(bucket.as_ref());
            }
        }
        None
    }
}

impl Factor {
    fn pass(&self, x: usize) -> bool {
        match *self {
            Factor::Under(n) => x < n,
            Factor::Equal(n) => x == n,
            Factor::Over(n) => x > n,
        }
    }
}
