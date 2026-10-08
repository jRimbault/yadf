mod common;

use common::{find_dupes, find_replicates, random_collection, AnyResult, TestDir, MAX_LEN};
use yadf::Factor;

/// Test to be sure the sorting by hash only groups together files
/// with the same contents.
/// Takes some time to run.
///
/// cargo test --package yadf --test common -- sanity_check --exact --nocapture -Z unstable-options --include-ignored
#[test]
#[ignore]
fn sanity_check() {
    let home = dirs::home_dir().unwrap();
    let counter = find_dupes(&home);
    for bucket in counter.duplicates().iter() {
        let (first, bucket) = bucket.split_first().unwrap();
        let reference = std::fs::read(first.to_path_buf()).unwrap();
        for file in bucket {
            let contents = std::fs::read(file.to_path_buf()).unwrap();
            assert_eq!(reference, contents, "comparing {first:?} and {file:?}");
        }
    }
}

#[test]
// #[ignore]
fn identical_small_files() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    println!("{:?}", root.as_ref());
    root.write_file("file1", b"aaa")?;
    root.write_file("file2", b"aaa")?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 1);
    assert_eq!(counter.as_inner().len(), 1);
    Ok(())
}

#[test]
// #[ignore]
fn identical_larger_files() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let buffer: Vec<_> = random_collection(MAX_LEN * 3);
    root.write_file("file1", &buffer)?;
    root.write_file("file2", &buffer)?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 1);
    assert_eq!(counter.as_inner().len(), 1);
    Ok(())
}

#[test]
// #[ignore]
fn files_differing_by_size() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    root.write_file("file1", b"aaaa")?;
    root.write_file("file2", b"aaa")?;
    assert_split_apart(&root);
    Ok(())
}

#[test]
// #[ignore]
fn files_differing_by_prefix() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    root.write_file("file1", b"aaa")?;
    root.write_file("file2", b"bbb")?;
    assert_split_apart(&root);
    Ok(())
}

#[test]
// #[ignore]
fn files_differing_by_suffix() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let mut buffer1 = Vec::with_capacity(MAX_LEN * 3 + 4);
    buffer1.extend_from_slice(&random_collection::<_, Vec<_>>(MAX_LEN * 3));
    let mut buffer2 = buffer1.clone();
    buffer1.extend_from_slice(b"suf1");
    buffer2.extend_from_slice(b"suf2");
    root.write_file("file1", &buffer1)?;
    root.write_file("file2", &buffer2)?;
    assert_split_apart(&root);
    Ok(())
}

#[test]
// #[ignore]
fn files_differing_by_middle() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let mut buffer1 = Vec::with_capacity(MAX_LEN * 2 + 4);
    buffer1.extend_from_slice(&random_collection::<_, Vec<_>>(MAX_LEN));
    let mut buffer2 = buffer1.clone();
    buffer1.extend_from_slice(b"mid1");
    buffer2.extend_from_slice(b"mid2");
    let suffix = random_collection::<_, Vec<_>>(MAX_LEN);
    buffer1.extend_from_slice(&suffix);
    buffer2.extend_from_slice(&suffix);
    root.write_file("file1", &buffer1)?;
    root.write_file("file2", &buffer2)?;
    assert_split_apart(&root);
    Ok(())
}

/// Two files whose contents differ end up as two unique buckets, and a
/// duplicates scan keeps neither.
fn assert_split_apart(root: &TestDir) {
    let uniques = find_replicates(root, Factor::Equal(1));
    assert_eq!(uniques.replicates(Factor::Equal(1)).iter().count(), 2);
    let counter = find_dupes(root);
    assert_eq!(counter.duplicates().iter().count(), 0);
}

/// Three copies of `a`, two of `b`, a unique `c`, and three large files of
/// one size where only two share a tail: every replication factor must see
/// the same groups whatever the pipeline pruned on the way.
#[test]
fn every_factor_reports_its_buckets() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    for name in ["a1", "a2", "a3"] {
        root.write_file(name, b"aaaa")?;
    }
    for name in ["b1", "b2"] {
        root.write_file(name, b"bbbb")?;
    }
    root.write_file("c", b"cc")?;
    let large: Vec<u8> = random_collection(MAX_LEN);
    let mut other = large.clone();
    *other.last_mut().unwrap() ^= 1;
    root.write_file("l1", &large)?;
    root.write_file("l2", &large)?;
    root.write_file("l3", &other)?;

    let groups = |factor| {
        let counter = find_replicates(&root, factor);
        let mut groups: Vec<Vec<String>> = counter
            .replicates(factor)
            .iter()
            .map(|bucket| {
                let mut names: Vec<_> = bucket
                    .iter()
                    .map(|path| {
                        let path = path.to_path_buf();
                        path.file_name().unwrap().to_string_lossy().into_owned()
                    })
                    .collect();
                names.sort();
                names
            })
            .collect();
        groups.sort();
        groups
    };

    assert_eq!(
        groups(Factor::Over(1)),
        [vec!["a1", "a2", "a3"], vec!["b1", "b2"], vec!["l1", "l2"]]
    );
    assert_eq!(groups(Factor::Over(2)), [vec!["a1", "a2", "a3"]]);
    assert_eq!(
        groups(Factor::Equal(2)),
        [vec!["b1", "b2"], vec!["l1", "l2"]]
    );
    assert_eq!(groups(Factor::Equal(1)), [vec!["c"], vec!["l3"]]);
    assert_eq!(
        groups(Factor::Under(3)),
        [vec!["b1", "b2"], vec!["c"], vec!["l1", "l2"], vec!["l3"]]
    );
    assert!(groups(Factor::Under(1)).is_empty());
    Ok(())
}
