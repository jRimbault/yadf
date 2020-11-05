use super::TreeBag;
use serde::ser::{Serialize, Serializer};

impl<H, T> Serialize for TreeBag<H, T>
where
    H: Ord,
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.duplicates().iter())
    }
}

#[cfg(test)]
mod tests {
    use super::super::TreeBag;

    #[test]
    fn json() {
        let counter: TreeBag<i32, &str> = vec![
            (77, "hello"),
            (77, "world"),
            (1, "ignored"),
            (3, "foo"),
            (3, "bar"),
        ]
        .into_iter()
        .collect();
        let result = serde_json::to_string(&counter).unwrap();
        let expected = r#"[["foo","bar"],["hello","world"]]"#;
        assert_eq!(result, expected);
    }
}
