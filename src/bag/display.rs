use super::{Display, Fdupes, Machine};
use std::fmt;

impl<'a, H: Ord, T> fmt::Display for Display<'a, H, T, Fdupes>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut duplicates = self.counter.duplicates().peekable();
        while let Some(bucket) = duplicates.next() {
            let mut bucket = bucket.iter().peekable();
            let is_last_bucket = duplicates.peek().is_none();
            while let Some(dupe) = bucket.next() {
                dupe.fmt(f)?;
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

impl<'a, H: Ord, T> fmt::Display for Display<'a, H, T, Machine>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut duplicates = self.counter.duplicates().peekable();
        while let Some(bucket) = duplicates.next() {
            let (last, rest) = bucket.split_last().unwrap();
            for dupe in rest {
                fmt::Debug::fmt(dupe, f)?;
                f.write_str(" ")?;
            }
            fmt::Debug::fmt(last, f)?;
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

    #[test]
    fn machine() {
        let counter: TreeBag<i32, &str> = vec![
            (77, "hello"),
            (77, "world"),
            (1, "ignored"),
            (3, "foo"),
            (3, "bar"),
        ]
        .into_iter()
        .collect();
        let result = counter.display::<Machine>().to_string();
        let expected = r#""foo" "bar"
"hello" "world""#;
        assert_eq!(result, expected);
    }

    #[test]
    fn fdupes() {
        let counter: TreeBag<i32, &str> = vec![
            (77, "hello"),
            (77, "world"),
            (1, "ignored"),
            (3, "foo"),
            (3, "bar"),
        ]
        .into_iter()
        .collect();
        let result = counter.display::<Fdupes>().to_string();
        let expected = "foo\nbar\n\nhello\nworld";
        assert_eq!(result, expected);
    }
}
