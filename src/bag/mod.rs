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
/// assert_eq!(bag[&3].len(), 2);
/// assert_eq!(bag[&6].len(), 1);
/// assert_eq!(bag[&3][0], "hello world");
/// ```
#[repr(transparent)]
pub struct TreeBag<H: Ord, T>(pub(crate) BTreeMap<H, Vec<T>>);

impl<H: Ord, T> TreeBag<H, T> {
    /// Provides a view on all the buckets containing more than one element.
    pub fn duplicates(&self) -> impl Iterator<Item = &Vec<T>> {
        self.values().filter(|b| b.len() > 1)
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

impl<H: Ord, T> std::ops::Deref for TreeBag<H, T> {
    type Target = BTreeMap<H, Vec<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Display marker.
#[derive(Debug)]
pub struct Machine;
#[derive(Debug)]
/// Display marker.
pub struct Fdupes;

pub struct Display<'a, H: Ord, T, U: marker::OutputFormat> {
    _marker: std::marker::PhantomData<U>,
    counter: &'a TreeBag<H, T>,
}

impl<H: Ord, T> TreeBag<H, T> {
    /// Returns an object that implements [`Display`](https://doc.rust-lang.org/stable/std/fmt/trait.Display.html).
    ///
    /// Depending on the contents of the [`TreeBag`](struct.TreeBag.html), the display object
    /// can be parameterized to get a different `Display` implemenation.
    pub fn display<D: marker::OutputFormat>(&self) -> Display<'_, H, T, D> {
        Display {
            counter: self,
            _marker: std::marker::PhantomData,
        }
    }
}

pub mod marker {
    pub trait OutputFormat {}
    impl OutputFormat for super::Fdupes {}
    impl OutputFormat for super::Machine {}
}
