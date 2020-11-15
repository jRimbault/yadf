use super::{Csv, Display, Fdupes, Machine};
use std::fmt;

impl<H: Ord, T> fmt::Display for Display<'_, H, T, Csv>
where
    T: serde::Serialize,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        #[derive(serde::Serialize)]
        struct CsvLine<'a, T: serde::Serialize> {
            count: usize,
            bucket: &'a [T],
        }

        let mut writer = csv::WriterBuilder::new()
            .flexible(true)
            .has_headers(false)
            .from_writer(FmtIoWriter(f));

        writer
            .write_record(&["count", "bucket"])
            .map_err(|_| fmt::Error)?;
        for bucket in self.duplicates.iter() {
            writer
                .serialize(CsvLine {
                    count: bucket.len(),
                    bucket,
                })
                .map_err(|_| fmt::Error)?;
        }
        Ok(())
    }
}

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

struct FmtIoWriter<'a, 'b>(&'a mut fmt::Formatter<'b>);

impl<'a, 'b> std::io::Write for FmtIoWriter<'a, 'b> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let _ = std::str::from_utf8(buf)
            .map(|s| self.0.write_str(s))
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
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
        let result = counter.duplicates().display::<Machine>().to_string();
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
        let result = counter.duplicates().display::<Fdupes>().to_string();
        let expected = "foo\nbar\n\nhello\nworld";
        assert_eq!(result, expected);
    }

    #[test]
    fn csv() {
        let counter: TreeBag<i32, &str> = vec![
            (77, "hello"),
            (77, "world"),
            (1, "ignored"),
            (3, "foo"),
            (3, "bar"),
        ]
        .into_iter()
        .collect();
        let result = counter.duplicates().display::<Csv>().to_string();
        let expected = "\
            count,bucket\n\
            2,foo,bar\n\
            2,hello,world\n\
        ";
        assert_eq!(result, expected);
    }
}
