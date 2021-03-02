mod display;
mod replicates;
mod serialize;

use std::borrow::Borrow;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::iter::FromIterator;
use std::ops::Index;

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
/// assert_eq!(bag[&3].len(), 2);
/// assert_eq!(bag[&6].len(), 1);
/// assert_eq!(bag[&3][0], "hello world");
/// ```
#[derive(Debug)]
pub struct TreeBag<K, V>(BTreeMap<K, Vec<V>>);

#[derive(Debug)]
pub enum Factor {
    Under(usize),
    Equal(usize),
    Over(usize),
}

/// A view which only provides access to n replicated entries
#[derive(Debug)]
pub struct Replicates<'a, K, V> {
    tree: &'a TreeBag<K, V>,
    factor: Factor,
}

/// Display marker.
#[derive(Debug)]
pub struct Fdupes;
/// Display marker.
#[derive(Debug)]
pub struct Machine;

#[derive(Debug)]
pub struct Display<'a, K, V, U> {
    _marker: std::marker::PhantomData<&'a U>,
    tree: &'a Replicates<'a, K, V>,
}

impl<K, V> From<BTreeMap<K, Vec<V>>> for TreeBag<K, V> {
    /// Build a `TreeBag` from a `BTreeMap`
    fn from(btree: BTreeMap<K, Vec<V>>) -> Self {
        Self(btree)
    }
}

impl<K, V> TreeBag<K, V> {
    /// Provides a view only on the buckets containing more than one element.
    pub const fn duplicates(&self) -> Replicates<'_, K, V> {
        Replicates {
            tree: self,
            factor: Factor::Over(1),
        }
    }

    pub const fn replicates(&self, factor: Factor) -> Replicates<'_, K, V> {
        Replicates { tree: self, factor }
    }

    /// Borrows the backing `BTreeMap` of the bag
    pub const fn as_inner(&self) -> &BTreeMap<K, Vec<V>> {
        &self.0
    }

    /// Mutably borrows the backing `BTreeMap` of the bag
    pub fn as_inner_mut(&mut self) -> &mut BTreeMap<K, Vec<V>> {
        &mut self.0
    }

    /// Consumes the wrapper `TreeBag` and returns the inner `BTreeMap`
    pub fn into_inner(self) -> BTreeMap<K, Vec<V>> {
        self.0
    }

    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&Vec<V>>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.0.get(key)
    }

    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut Vec<V>>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.0.get_mut(key)
    }

    /// Gets the given key’s corresponding entry in the map for in-place manipulation.
    pub fn entry(&mut self, key: K) -> Entry<'_, K, Vec<V>>
    where
        K: Ord,
    {
        self.0.entry(key)
    }
}

impl<K: Ord, V> FromIterator<(K, V)> for TreeBag<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(key_value_iter: I) -> Self {
        let mut bag = TreeBag(BTreeMap::default());
        for (key, value) in key_value_iter {
            bag.entry(key).or_default().push(value);
        }
        bag
    }
}

impl<K, Q: ?Sized, V> Index<&Q> for TreeBag<K, V>
where
    K: Borrow<Q> + Ord,
    Q: Ord,
{
    type Output = Vec<V>;

    /// Returns a reference to the value corresponding to the supplied key.
    ///
    /// # Panics
    ///
    /// Panics if the key is not present in the `TreeBag`.
    fn index(&self, key: &Q) -> &Self::Output {
        self.get(key).expect("no entry found for key")
    }
}
