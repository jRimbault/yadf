mod common;

use common::{find_dupes, random_collection, AnyResult, TestDir, MAX_LEN};

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
        let reference = std::fs::read(&first).unwrap();
        for file in bucket {
            let contents = std::fs::read(&file).unwrap();
            assert_eq!(reference, contents, "comparing {:?} and {:?}", first, file);
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
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_inner().len(), 2);
    Ok(())
}

#[test]
// #[ignore]
fn files_differing_by_prefix() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    root.write_file("file1", b"aaa")?;
    root.write_file("file2", b"bbb")?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_inner().len(), 2);
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
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_inner().len(), 2);
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
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_inner().len(), 2);
    Ok(())
}
