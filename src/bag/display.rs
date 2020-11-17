use super::{Display, Fdupes, Machine};
use std::fmt;

impl<H: Ord, T> fmt::Display for Display<'_, H, T, Fdupes>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut duplicates = self.duplicates.iter().peekable();
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

impl<H: Ord, T> fmt::Display for Display<'_, H, T, Machine>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut duplicates = self.duplicates.iter().peekable();
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

    lazy_static::lazy_static! {
        static ref BAG: TreeBag<i32, &'static str> = vec![
            (77, "hello"),
            (77, "world"),
            (1, "ignored"),
            (3, "foo"),
            (3, "bar"),
        ]
        .into_iter()
        .collect();
    }

    #[test]
    fn machine() {
        let result = BAG.duplicates().display::<Machine>().to_string();
        let expected = "\
            \"foo\" \"bar\"\n\
            \"hello\" \"world\"\
        ";
        assert_eq!(result, expected);
    }

    #[test]
    fn fdupes() {
        let result = BAG.duplicates().display::<Fdupes>().to_string();
        let expected = "\
            foo\n\
            bar\n\
            \n\
            hello\n\
            world\
        ";
        assert_eq!(result, expected);
    }
}
