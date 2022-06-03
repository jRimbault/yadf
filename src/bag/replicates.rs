use super::{Display, Factor, Replicates};

/// [`Iterator`](Iterator) adapater.
#[derive(Debug)]
pub struct Iter<'a, K, V> {
    values: indexmap::map::Values<'a, K, Vec<V>>,
    factor: Factor,
}

impl<K, V> Replicates<'_, K, V> {
    /// Iterator over the buckets.
    pub fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            values: self.tree.0.values(),
            factor: self.factor.clone(),
        }
    }

    /// Returns an object that implements [`Display`](std::fmt::Display).
    ///
    /// Depending on the contents of the [`TreeBag`](super::TreeBag), the display object
    /// can be parameterized to get a different [`Display`](std::fmt::Display) implemenation.
    pub fn display<U>(&self) -> Display<'_, K, V, U> {
        Display {
            format_marker: std::marker::PhantomData,
            tree: self,
        }
    }
}

impl<'a, K, V> IntoIterator for &'a Replicates<'a, K, V> {
    type Item = &'a Vec<V>;
    type IntoIter = Iter<'a, K, V>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, K, V> Iterator for Iter<'a, K, V> {
    type Item = &'a Vec<V>;

    fn next(&mut self) -> Option<Self::Item> {
        for bucket in &mut self.values {
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
