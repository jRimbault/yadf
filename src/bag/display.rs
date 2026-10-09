use super::{Display, Fdupes, Machine};
use std::fmt;

/// Like `fdupes`: every value ends with a newline, and every bucket with
/// one more, the last one included.
impl<K, V> fmt::Display for Display<'_, K, V, Fdupes>
where
    V: fmt::Display + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for bucket in self.tree.iter() {
            for dupe in bucket {
                fmt::Display::fmt(dupe, f)?;
                f.write_str("\n")?;
            }
            f.write_str("\n")?;
        }
        Ok(())
    }
}

/// Every value ends with a NUL, and every bucket with one more: an empty
/// record, which no path can be.
impl<K, V> fmt::Display for Display<'_, K, V, Machine>
where
    V: fmt::Display + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for bucket in self.tree.iter() {
            for dupe in bucket {
                fmt::Display::fmt(dupe, f)?;
                f.write_str("\0")?;
            }
            f.write_str("\0")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::TreeBag;
    use super::*;
    use once_cell::sync::Lazy;

    static BAG: Lazy<TreeBag<i32, &'static str>> = Lazy::new(|| {
        vec![
            (77, "hello"),
            (77, "world"),
            (1, "ignored"),
            (3, "foo"),
            (3, "bar"),
        ]
        .into_iter()
        .collect()
    });

    #[test]
    fn machine() {
        let result = BAG.duplicates().display::<Machine>().to_string();
        let expected = "foo\0bar\0\0hello\0world\0\0";
        assert_eq!(result, expected);
    }

    #[test]
    fn fdupes() {
        let result = BAG.duplicates().display::<Fdupes>().to_string();
        let expected = "foo\nbar\n\nhello\nworld\n\n";
        assert_eq!(result, expected);
    }
}
