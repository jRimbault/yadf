mod display;
mod serialize;

use std::collections::BTreeMap;

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
/// assert_eq!(bag.as_tree()[&3].len(), 2);
/// assert_eq!(bag.as_tree()[&6].len(), 1);
/// assert_eq!(bag.as_tree()[&3][0], "hello world");
/// ```
#[repr(transparent)]
#[derive(Debug)]
pub struct TreeBag<H: Ord, T>(pub(crate) BTreeMap<H, Vec<T>>);

#[derive(Debug, Clone, Copy)]
pub enum Factor {
    Under(usize),
    Equal(usize),
    Over(usize),
}

/// A view which only provides access to n replicated entries
#[derive(Debug)]
pub struct Replicates<'a, H: Ord, T> {
    tree: &'a TreeBag<H, T>,
    factor: Factor,
}

/// Display marker.
#[derive(Debug)]
pub struct Fdupes;
/// Display marker.
#[derive(Debug)]
pub struct Machine;

#[derive(Debug)]
pub struct Display<'a, H: Ord, T, U: marker::OutputFormat> {
    _marker: std::marker::PhantomData<&'a U>,
    tree: &'a Replicates<'a, H, T>,
}

impl<H: Ord, T> TreeBag<H, T> {
    /// Provides a view only on the buckets containing more than one element.
    pub fn duplicates(&self) -> Replicates<'_, H, T> {
        Replicates {
            tree: self,
            factor: Factor::Over(1),
        }
    }

    pub fn replicates(&self, factor: Factor) -> Replicates<'_, H, T> {
        Replicates { tree: self, factor }
    }

    /// Borrows the backing tree map of the bag
    pub fn as_tree(&self) -> &BTreeMap<H, Vec<T>> {
        &self.0
    }
}

impl<H: Ord, T> Replicates<'_, H, T> {
    /// Iterator over the buckets
    pub fn iter(&self) -> impl Iterator<Item = &[T]> {
        self.tree
            .0
            .values()
            .filter(move |bucket| match self.factor {
                Factor::Under(n) => bucket.len() < n,
                Factor::Equal(n) => bucket.len() == n,
                Factor::Over(n) => bucket.len() > n,
            })
            .map(AsRef::as_ref)
    }

    /// Returns an object that implements [`Display`](std::fmt::Display)
    ///
    /// Depending on the contents of the [`TreeBag`](TreeBag), the display object
    /// can be parameterized to get a different `Display` implemenation.
    pub fn display<D: marker::OutputFormat>(&self) -> Display<'_, H, T, D> {
        Display {
            tree: self,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<H: Ord, T> std::iter::FromIterator<(H, T)> for TreeBag<H, T> {
    fn from_iter<I: IntoIterator<Item = (H, T)>>(iter: I) -> Self {
        let mut map: BTreeMap<H, Vec<T>> = BTreeMap::new();
        for (hash, item) in iter {
            map.entry(hash).or_default().push(item);
        }
        Self(map)
    }
}

impl<H: Ord, T> AsRef<BTreeMap<H, Vec<T>>> for TreeBag<H, T> {
    fn as_ref(&self) -> &BTreeMap<H, Vec<T>> {
        self.as_tree()
    }
}

impl<H: Ord, T> From<TreeBag<H, T>> for BTreeMap<H, Vec<T>> {
    fn from(value: TreeBag<H, T>) -> Self {
        value.0
    }
}

impl<H: Ord, T> From<BTreeMap<H, Vec<T>>> for TreeBag<H, T> {
    fn from(value: BTreeMap<H, Vec<T>>) -> Self {
        Self(value)
    }
}

pub mod marker {
    pub trait OutputFormat {}
    impl OutputFormat for super::Fdupes {}
    impl OutputFormat for super::Machine {}
}
