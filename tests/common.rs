/// Test to be sure the sorting by hash only groups together files
/// with the same contents.
/// Takes some time to run.
///
/// cargo test --package yadf --test common -- sanity_test --exact --nocapture -Z unstable-options --include-ignored
#[test]
#[ignore]
fn sanity_test() {
    let hasher: std::marker::PhantomData<yadf::SeaHasher> = Default::default();
    let home = dirs::home_dir().unwrap();
    let counter = yadf::find_dupes(hasher, &[home], None, None);
    for bucket in counter.duplicates() {
        let (first, bucket) = bucket.split_first().unwrap();
        let reference = std::fs::read(first.path()).unwrap();
        for file in bucket {
            let contents = std::fs::read(file.path()).unwrap();
            assert_eq!(
                reference,
                contents,
                "comparing {:?} and {:?}",
                first.path(),
                file.path()
            );
        }
    }
}
