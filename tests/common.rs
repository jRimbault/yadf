mod test_dir;

use test_dir::TestDir;

/// quick-n-dirty any result type alias
type AnyResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const MAX_LEN: usize = 256 * 1024;

/// Test to be sure the sorting by hash only groups together files
/// with the same contents.
/// Takes some time to run.
///
/// cargo test --package yadf --test common -- sanity_test --exact --nocapture -Z unstable-options --include-ignored
#[test]
#[ignore]
fn sanity_test() {
    let home = dirs::home_dir().unwrap();
    let counter = yadf::Config::builder()
        .paths(&[home])
        .build()
        .scan::<yadf::SeaHasher>();
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

#[cfg(windows)]
macro_rules! DIR {
    ($name:ty) => {{
        concat!("target", "\\", file!(), "\\", stringify!($name))
    }};
}

#[cfg(not(windows))]
macro_rules! DIR {
    ($name:ty) => {
        concat!("target", "/", file!(), "/", stringify!($name))
    };
}

/// test shortcut
fn find_dupes<P: AsRef<std::path::Path>>(path: &P) -> yadf::TreeBag<u64, yadf::DirEntry> {
    yadf::Config::builder()
        .paths(&[path])
        .build()
        .scan::<yadf::SeaHasher>()
}

#[test]
fn identical_small_files() -> AnyResult {
    let root = TestDir::try_new(&DIR!(identical_small_files))?;
    println!("{:?}", root.as_ref());
    root.write_file_in_three_parts(&"file1", b"aaa", b"", b"")?;
    root.write_file_in_three_parts(&"file2", b"aaa", b"", b"")?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 1);
    assert_eq!(counter.as_tree().len(), 1);
    Ok(())
}

#[test]
fn identical_larger_files() -> AnyResult {
    let root = TestDir::try_new(&DIR!(identical_larger_files))?;
    let prefix = [0; MAX_LEN];
    let middle = [1; MAX_LEN];
    let suffix = [2; MAX_LEN];
    root.write_file_in_three_parts(&"file1", &prefix, &middle, &suffix)?;
    root.write_file_in_three_parts(&"file2", &prefix, &middle, &suffix)?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 1);
    assert_eq!(counter.as_tree().len(), 1);
    Ok(())
}

#[test]
fn files_differing_by_size() -> AnyResult {
    let root = TestDir::try_new(&DIR!(files_differing_by_size))?;
    root.write_file_in_three_parts(&"file1", b"aaaa", b"", b"")?;
    root.write_file_in_three_parts(&"file2", b"aaa", b"", b"")?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_tree().len(), 2);
    Ok(())
}

#[test]
fn files_differing_by_prefix() -> AnyResult {
    let root = TestDir::try_new(&DIR!(files_differing_by_prefix))?;
    root.write_file_in_three_parts(&"file1", b"aaa", b"", b"")?;
    root.write_file_in_three_parts(&"file2", b"bbb", b"", b"")?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_tree().len(), 2);
    Ok(())
}

#[test]
fn files_differing_by_suffix() -> AnyResult {
    let root = TestDir::try_new(&DIR!(files_differing_by_suffix))?;
    let prefix = [0; MAX_LEN];
    let middle = [1; MAX_LEN * 2];
    root.write_file_in_three_parts(&"file1", &prefix, &middle, b"suf1")?;
    root.write_file_in_three_parts(&"file2", &prefix, &middle, b"suf2")?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_tree().len(), 2);
    Ok(())
}

#[test]
fn files_differing_by_middle() -> AnyResult {
    let root = TestDir::try_new(&DIR!(files_differing_by_middle))?;
    let prefix = [0; MAX_LEN];
    let suffix = [1; MAX_LEN];
    root.write_file_in_three_parts(&"file1", &prefix, b"mid1", &suffix)?;
    root.write_file_in_three_parts(&"file2", &prefix, b"mid2", &suffix)?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_tree().len(), 2);
    Ok(())
}

mod integration {
    use super::test_dir::TestDir;
    use super::{AnyResult, MAX_LEN};

    fn random_vec<T>(size: usize) -> Vec<T>
    where
        rand::distributions::Standard: rand::distributions::Distribution<T>,
    {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        std::iter::repeat(())
            .map(|_| rng.gen())
            .take(size)
            .collect()
    }

    #[test]
    fn debug_output() -> AnyResult {
        use predicates::prelude::*;
        let root = TestDir::try_new(&DIR!(debug_output))?;
        let bytes: Vec<u8> = random_vec(MAX_LEN);
        let file1 = root.write_file(&"file1", &bytes)?;
        let file2 = root.write_file(&"file2", &bytes)?;
        root.write_file(&"file3", &bytes[..4096])?;
        root.write_file(&"file4", &bytes[..2048])?;
        let expected1 =
            serde_json::to_string(&[[file1.to_string_lossy(), file2.to_string_lossy()]]).unwrap();
        let expected2 =
            serde_json::to_string(&[[file2.to_string_lossy(), file1.to_string_lossy()]]).unwrap();
        assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
            .arg("-vvv") // test stderr contains enough debug output
            .args(&["--format", "json"])
            .args(&["--algorithm", "seahash"])
            .arg(root.as_ref())
            .assert()
            .success()
            .stderr(predicate::str::contains("started with Args {").from_utf8())
            .stderr(predicate::str::contains("format: Json").from_utf8())
            .stderr(predicate::str::contains("algorithm: SeaHash").from_utf8())
            .stderr(predicate::str::contains("verbose: 3").from_utf8())
            .stderr(
                predicate::str::contains("found 3 possible duplicates after initial scan")
                    .from_utf8(),
            )
            .stderr(
                predicate::str::contains("found 2 duplicates in 1 groups after checksumming")
                    .from_utf8(),
            )
            .stdout(
                predicate::str::contains(expected1)
                    .from_utf8()
                    .or(predicate::str::contains(expected2).from_utf8()),
            );
        Ok(())
    }

    #[test]
    fn regex() -> AnyResult {
        use predicates::prelude::*;
        let root = TestDir::try_new(&DIR!(regex))?;
        let bytes: Vec<u8> = random_vec(4096);
        let particular_1_name = root.write_file(&"particular_1_name", &bytes)?;
        let particular_2_name = root.write_file(&"particular_2_name", &bytes)?;
        root.write_file(&"not_particular_2_name", &bytes)?;
        root.write_file(&"completely_different", &bytes)?;
        let expected1 = [
            particular_1_name.to_string_lossy(),
            particular_2_name.to_string_lossy(),
        ]
        .join("\n")
            + "\n";
        let expected2 = [
            particular_2_name.to_string_lossy(),
            particular_1_name.to_string_lossy(),
        ]
        .join("\n")
            + "\n";
        assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
            .args(&["--regex", "^particular_\\d_name$"])
            .arg(root.as_ref())
            .assert()
            .stdout(
                predicate::str::similar(expected1)
                    .from_utf8()
                    .or(predicate::str::similar(expected2).from_utf8()),
            );
        Ok(())
    }

    #[test]
    fn glob_pattern() -> AnyResult {
        use predicates::prelude::*;
        let root = TestDir::try_new(&DIR!(glob_pattern))?;
        let bytes: Vec<u8> = random_vec(4096);
        let particular_1_name = root.write_file(&"particular_1_name", &bytes)?;
        let particular_2_name = root.write_file(&"particular_2_name", &bytes)?;
        root.write_file(&"not_particular_2_name", &bytes)?;
        root.write_file(&"completely_different", &bytes)?;
        let expected1 = [
            particular_1_name.to_string_lossy(),
            particular_2_name.to_string_lossy(),
        ]
        .join("\n")
            + "\n";
        let expected2 = [
            particular_2_name.to_string_lossy(),
            particular_1_name.to_string_lossy(),
        ]
        .join("\n")
            + "\n";
        assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
            .args(&["--pattern", "particular*name"])
            .arg(root.as_ref())
            .assert()
            .stdout(
                predicate::str::similar(expected1)
                    .from_utf8()
                    .or(predicate::str::similar(expected2).from_utf8()),
            );
        Ok(())
    }

    #[test]
    fn min_file_size() -> AnyResult {
        use predicates::prelude::*;
        let root = TestDir::try_new(&DIR!(min_file_size))?;
        let bytes: Vec<u8> = random_vec(4096);
        let particular_1_name = root.write_file(&"particular_1_name", &bytes)?;
        let particular_2_name = root.write_file(&"particular_2_name", &bytes)?;
        root.write_file(&"not_particular_2_name", &bytes[..2048])?;
        root.write_file(&"completely_different", &bytes[..2048])?;
        let expected1 = [
            particular_1_name.to_string_lossy(),
            particular_2_name.to_string_lossy(),
        ]
        .join("\n")
            + "\n";
        let expected2 = [
            particular_2_name.to_string_lossy(),
            particular_1_name.to_string_lossy(),
        ]
        .join("\n")
            + "\n";
        assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
            .args(&["--min", "4K"])
            .arg(root.as_ref())
            .assert()
            .stdout(
                predicate::str::similar(expected1)
                    .from_utf8()
                    .or(predicate::str::similar(expected2).from_utf8()),
            );
        Ok(())
    }

    #[test]
    fn max_file_size() -> AnyResult {
        use predicates::prelude::*;
        let root = TestDir::try_new(&DIR!(max_file_size))?;
        let bytes: Vec<u8> = random_vec(4096);
        let particular_1_name = root.write_file(&"particular_1_name", &bytes[..1024])?;
        let particular_2_name = root.write_file(&"particular_2_name", &bytes[..1024])?;
        root.write_file(&"not_particular_2_name", &bytes)?;
        root.write_file(&"completely_different", &bytes)?;
        let expected1 = [
            particular_1_name.to_string_lossy(),
            particular_2_name.to_string_lossy(),
        ]
        .join("\n")
            + "\n";
        let expected2 = [
            particular_2_name.to_string_lossy(),
            particular_1_name.to_string_lossy(),
        ]
        .join("\n")
            + "\n";
        assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
            .args(&["--max", "2K"])
            .arg(root.as_ref())
            .assert()
            .stdout(
                predicate::str::similar(expected1)
                    .from_utf8()
                    .or(predicate::str::similar(expected2).from_utf8()),
            );
        Ok(())
    }
}
