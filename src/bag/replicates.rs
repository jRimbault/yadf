use super::{Display, Factor, Replicates};
use std::collections::btree_map::Values;

/// [`Iterator`] adapter.
#[derive(Debug)]
pub struct Iter<'a, K, V> {
    values: Values<'a, K, Vec<V>>,
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
    /// can be parameterized to get a different [`Display`](std::fmt::Display) implementation.
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

#[allow(clippy::manual_find)]
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

#[cfg(test)]
mod tests {
    use super::super::TreeBag;
    use super::Factor;

    fn sample() -> TreeBag<i32, &'static str> {
        vec![(1, "a"), (2, "b"), (2, "c"), (3, "d"), (3, "e"), (3, "f")]
            .into_iter()
            .collect()
    }

    #[test]
    fn into_iter_for_ref_replicates() {
        let bag = sample();
        let duplicates = bag.duplicates();
        let buckets: Vec<_> = (&duplicates).into_iter().collect();
        assert_eq!(buckets, vec![&vec!["b", "c"], &vec!["d", "e", "f"]]);
    }

    #[test]
    fn factor_under() {
        let bag = sample();
        let replicates = bag.replicates(Factor::Under(2));
        let buckets: Vec<_> = replicates.iter().collect();
        assert_eq!(buckets, vec![&vec!["a"]]);
    }

    #[test]
    fn factor_equal() {
        let bag = sample();
        let replicates = bag.replicates(Factor::Equal(2));
        let buckets: Vec<_> = replicates.iter().collect();
        assert_eq!(buckets, vec![&vec!["b", "c"]]);
    }
}
