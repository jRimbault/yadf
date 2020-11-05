mod test_dir;

use std::marker::PhantomData;
use test_dir::TestDir;

const MAX_LEN: usize = 256 * 1024;

/// Test to be sure the sorting by hash only groups together files
/// with the same contents.
/// Takes some time to run.
///
/// cargo test --package yadf --test common -- sanity_test --exact --nocapture -Z unstable-options --include-ignored
#[test]
#[ignore]
fn sanity_test() {
    let hasher: PhantomData<yadf::SeaHasher> = Default::default();
    let home = dirs::home_dir().unwrap();
    let counter = yadf::find_dupes(hasher, yadf::SearchConfig::builder().paths(&[home]).build());
    for bucket in counter.duplicates().iter() {
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

macro_rules! DIR {
    ($name:ty) => {
        concat!("target/", file!(), "/", stringify!($name))
    };
}

/// test shortcut
fn find_dupes<P: AsRef<std::path::Path>>(path: &P) -> yadf::TreeBag<u64, yadf::DirEntry> {
    let hasher: PhantomData<yadf::SeaHasher> = Default::default();
    yadf::find_dupes(hasher, yadf::SearchConfig::builder().paths(&[path]).build())
}

#[test]
fn identical_small_files() -> std::io::Result<()> {
    let root = TestDir::try_new(&DIR!(identical_small_files))?;
    println!("{:?}", root.as_ref());
    assert!(root.write_file(&"file1", b"aaa", b"", b"").is_ok());
    assert!(root.write_file(&"file2", b"aaa", b"", b"").is_ok());
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 1);
    assert_eq!(counter.len(), 1);
    Ok(())
}

#[test]
fn identical_larger_files() -> std::io::Result<()> {
    let root = TestDir::try_new(&DIR!(identical_larger_files))?;
    let prefix = [0; MAX_LEN];
    let middle = [1; MAX_LEN];
    let suffix = [2; MAX_LEN];
    assert!(root.write_file(&"file1", &prefix, &middle, &suffix).is_ok());
    assert!(root.write_file(&"file2", &prefix, &middle, &suffix).is_ok());
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 1);
    assert_eq!(counter.len(), 1);
    Ok(())
}

#[test]
fn files_differing_by_size() -> std::io::Result<()> {
    let root = TestDir::try_new(&DIR!(files_differing_by_size))?;
    assert!(root.write_file(&"file1", b"aaaa", b"", b"").is_ok());
    assert!(root.write_file(&"file2", b"aaa", b"", b"").is_ok());
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.len(), 2);
    Ok(())
}

#[test]
fn files_differing_by_prefix() -> std::io::Result<()> {
    let root = TestDir::try_new(&DIR!(files_differing_by_prefix))?;
    assert!(root.write_file(&"file1", b"aaa", b"", b"").is_ok());
    assert!(root.write_file(&"file2", b"bbb", b"", b"").is_ok());
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.len(), 2);
    Ok(())
}

#[test]
fn files_differing_by_suffix() -> std::io::Result<()> {
    let root = TestDir::try_new(&DIR!(files_differing_by_suffix))?;
    let prefix = [0; MAX_LEN];
    let middle = [1; MAX_LEN * 2];
    assert!(root.write_file(&"file1", &prefix, &middle, b"suf1").is_ok());
    assert!(root.write_file(&"file2", &prefix, &middle, b"suf2").is_ok());
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.len(), 2);
    Ok(())
}

#[test]
fn files_differing_by_middle() -> std::io::Result<()> {
    let root = TestDir::try_new(&DIR!(files_differing_by_middle))?;
    let prefix = [0; MAX_LEN];
    let suffix = [1; MAX_LEN];
    assert!(root.write_file(&"file1", &prefix, b"mid1", &suffix).is_ok());
    assert!(root.write_file(&"file2", &prefix, b"mid2", &suffix).is_ok());
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.len(), 2);
    Ok(())
}
