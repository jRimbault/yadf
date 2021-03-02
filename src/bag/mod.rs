mod display;
mod replicates;
mod serialize;

use std::borrow::Borrow;
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::iter::FromIterator;
use std::ops::Index;

/// Ordered counter structure.
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
///
/// assert_eq!(bag[&3], ["hello world", "foobar"]);
/// assert_eq!(bag[&7], ["fizz", "buzz"]);
/// assert_eq!(bag[&6], ["rust"]);
/// ```
#[derive(Debug)]
pub struct TreeBag<K, V>(BTreeMap<K, Vec<V>>);

#[derive(Debug, Clone)]
pub enum Factor {
    Under(usize),
    Equal(usize),
    Over(usize),
}

/// A view which only provides access to n replicated entries.
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
    _format_marker: std::marker::PhantomData<U>,
    tree: &'a Replicates<'a, K, V>,
}

impl<K, V> From<BTreeMap<K, Vec<V>>> for TreeBag<K, V> {
    /// Build a [`TreeBag`](TreeBag) from a [`BTreeMap`](BTreeMap).
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

    /// Provides a view only on the buckets as constrained by the replication [`Factor`](Factor).
    pub const fn replicates(&self, factor: Factor) -> Replicates<'_, K, V> {
        Replicates { tree: self, factor }
    }

    /// Borrows the backing [`BTreeMap`](BTreeMap) of the bag.
    pub const fn as_inner(&self) -> &BTreeMap<K, Vec<V>> {
        &self.0
    }

    /// Mutably borrows the backing [`BTreeMap`](BTreeMap) of the bag.
    pub fn as_inner_mut(&mut self) -> &mut BTreeMap<K, Vec<V>> {
        &mut self.0
    }

    /// Consumes the wrapper [`TreeBag`](TreeBag) and returns the inner [`BTreeMap`](BTreeMap).
    pub fn into_inner(self) -> BTreeMap<K, Vec<V>> {
        self.0
    }

    /// Returns the number of buckets in the bag.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if the bag contains no elements.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns a reference to the bucket corresponding to the key.
    pub fn get<Q: ?Sized>(&self, key: &Q) -> Option<&Vec<V>>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.0.get(key)
    }

    /// Returns a mutable reference to the bucket corresponding to the key.
    pub fn get_mut<Q: ?Sized>(&mut self, key: &Q) -> Option<&mut Vec<V>>
    where
        K: Borrow<Q> + Ord,
        Q: Ord,
    {
        self.0.get_mut(key)
    }

    /// Gets the given key’s corresponding entry in the bag for in-place manipulation.
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
    /// Panics if the key is not present in the [`TreeBag`](TreeBag).
    fn index(&self, key: &Q) -> &Self::Output {
        self.get(key).expect("no entry found for key")
    }
}
