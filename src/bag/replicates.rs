use super::{Display, Factor, Replicates};
use std::collections::btree_map::Values;

#[derive(Debug)]
pub struct Iter<'a, K, V> {
    values: Values<'a, K, Vec<V>>,
    factor: &'a Factor,
}

impl<K, V> Replicates<'_, K, V> {
    /// Iterator over the buckets
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            values: self.tree.0.values(),
            factor: &self.factor,
        }
    }

    /// Returns an object that implements [`Display`](std::fmt::Display)
    ///
    /// Depending on the contents of the [`TreeBag`](TreeBag), the display object
    /// can be parameterized to get a different `Display` implemenation.
    pub fn display<D>(&self) -> Display<'_, K, V, D> {
        Display {
            _marker: std::marker::PhantomData,
            tree: self,
        }
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = &'a Vec<V>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(bucket) = self.values.next() {
            if self.factor.pass(bucket.len()) {
                return Some(bucket);
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
