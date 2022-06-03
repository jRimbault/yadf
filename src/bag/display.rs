use super::{Display, Fdupes, Machine};
use std::fmt;
use std::path::Path;

impl<K, V> fmt::Display for Display<'_, K, V, Fdupes>
where
    V: AsRef<Path>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut duplicates = self.tree.iter().peekable();
        while let Some(bucket) = duplicates.next() {
            let mut bucket = bucket.iter().peekable();
            let is_last_bucket = duplicates.peek().is_none();
            while let Some(dupe) = bucket.next() {
                dupe.as_ref().display().fmt(f)?;
                if bucket.peek().is_some() || !is_last_bucket {
                    f.write_str("\n")?;
                }
            }
            if !is_last_bucket {
                f.write_str("\n")?;
            }
        }
        Ok(())
    }
}

impl<K, V> fmt::Display for Display<'_, K, V, Machine>
where
    V: AsRef<Path>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut duplicates = self.tree.iter().peekable();
        while let Some(bucket) = duplicates.next() {
            let (last, rest) = bucket.split_last().ok_or(fmt::Error)?;
            for dupe in rest {
                fmt::Debug::fmt(dupe.as_ref(), f)?;
                f.write_str(" ")?;
            }
            fmt::Debug::fmt(last.as_ref(), f)?;
            if duplicates.peek().is_some() {
                f.write_str("\n")?;
            }
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
        let expected = "\
            \"hello\" \"world\"\n\
            \"foo\" \"bar\"\
        ";
        assert_eq!(result, expected);
    }

    #[test]
    fn fdupes() {
        let result = BAG.duplicates().display::<Fdupes>().to_string();
        let expected = "\
            hello\n\
            world\n\
            \n\
            foo\n\
            bar\
        ";
        assert_eq!(result, expected);
    }
}
