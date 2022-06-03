use super::{Replicates, TreeBag};
use serde::ser::{Serialize, Serializer};

impl<K, V> Serialize for Replicates<'_, K, V>
where
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_seq(self.iter())
    }
}

impl<K, V> Serialize for TreeBag<K, V>
where
    K: Serialize,
    V: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_map(self.0.iter())
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
        let result = serde_json::to_string(&counter.duplicates()).unwrap();
        let expected = r#"[["hello","world"],["foo","bar"]]"#;
        assert_eq!(result, expected);
    }
}
