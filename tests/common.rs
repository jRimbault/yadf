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

macro_rules! test_dir {
    () => {{
        #[cfg(windows)]
        const SEP: &str = "\\";
        #[cfg(not(windows))]
        const SEP: &str = "/";
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        // `3` is the length of the `::f`.
        let name = &name[..name.len() - 3].replace("::", SEP);
        format!("target{}tests{}{}", SEP, SEP, name)
    }};
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
    let root = TestDir::new(test_dir!())?;
    println!("{:?}", root.as_ref());
    root.write_file(&"file1", b"aaa")?;
    root.write_file(&"file2", b"aaa")?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 1);
    assert_eq!(counter.as_tree().len(), 1);
    Ok(())
}

#[test]
fn identical_larger_files() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let mut buffer = Vec::with_capacity(MAX_LEN * 3);
    buffer.extend_from_slice(&[0; MAX_LEN]);
    buffer.extend_from_slice(&[1; MAX_LEN]);
    buffer.extend_from_slice(&[2; MAX_LEN]);
    root.write_file(&"file1", &buffer)?;
    root.write_file(&"file2", &buffer)?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 1);
    assert_eq!(counter.as_tree().len(), 1);
    Ok(())
}

#[test]
fn files_differing_by_size() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    root.write_file(&"file1", b"aaaa")?;
    root.write_file(&"file2", b"aaa")?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_tree().len(), 2);
    Ok(())
}

#[test]
fn files_differing_by_prefix() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    root.write_file(&"file1", b"aaa")?;
    root.write_file(&"file2", b"bbb")?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_tree().len(), 2);
    Ok(())
}

#[test]
fn files_differing_by_suffix() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let mut buffer1 = Vec::with_capacity(MAX_LEN * 3 + 4);
    buffer1.extend_from_slice(&[0; MAX_LEN]);
    buffer1.extend_from_slice(&[1; MAX_LEN * 2]);
    let mut buffer2 = buffer1.clone();
    buffer1.extend_from_slice(b"suf1");
    buffer2.extend_from_slice(b"suf2");
    root.write_file(&"file1", &buffer1)?;
    root.write_file(&"file2", &buffer2)?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_tree().len(), 2);
    Ok(())
}

#[test]
fn files_differing_by_middle() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let mut buffer1 = Vec::with_capacity(MAX_LEN * 2 + 4);
    buffer1.extend_from_slice(&[0; MAX_LEN]);
    let mut buffer2 = buffer1.clone();
    buffer1.extend_from_slice(b"mid1");
    buffer2.extend_from_slice(b"mid2");
    let suffix = [1; MAX_LEN];
    buffer1.extend_from_slice(&suffix);
    buffer2.extend_from_slice(&suffix);
    root.write_file(&"file1", &buffer1)?;
    root.write_file(&"file2", &buffer2)?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_tree().len(), 2);
    Ok(())
}

mod integration {
    use super::test_dir::TestDir;
    use super::{AnyResult, MAX_LEN};
    use predicates::{boolean::PredicateBooleanExt, str as predstr};

    fn random_collection<T, I>(size: usize) -> I
    where
        rand::distributions::Standard: rand::distributions::Distribution<T>,
        I: std::iter::FromIterator<T>,
    {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        std::iter::repeat_with(|| rng.gen()).take(size).collect()
    }

    #[test]
    fn trace_output() -> AnyResult {
        let root = TestDir::new(test_dir!())?;
        println!("{:?}", root.as_ref());
        let bytes: Vec<_> = random_collection(MAX_LEN);
        let file1 = root.write_file(&"file1", &bytes)?;
        let file2 = root.write_file(&"file2", &bytes)?;
        root.write_file(&"file3", &bytes[..4096])?;
        root.write_file(&"file4", &bytes[..2048])?;
        let expected = serde_json::to_string(&[[file1.to_string_lossy(), file2.to_string_lossy()]])
            .unwrap()
            + "\n";
        assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
            .arg("-vvvv") // test stderr contains enough debug output
            .args(&["--format", "json"])
            .args(&["--algorithm", "seahash"])
            .arg(root.as_ref())
            .assert()
            .success()
            .stderr(
                predstr::contains("Args {")
                    .and(predstr::contains("Config {"))
                    .and(predstr::contains("format: Json"))
                    .and(predstr::contains("algorithm: SeaHash"))
                    .and(predstr::contains("verbose: 4"))
                    .and(predstr::contains(
                        "found 3 possible duplicates after initial scan",
                    ))
                    .and(predstr::contains(
                        "found 2 duplicates in 1 groups after checksumming",
                    ))
                    .and(predstr::contains("file1"))
                    .and(predstr::contains("file2"))
                    .and(predstr::contains("file3"))
                    .and(predstr::contains("file4")),
            )
            .stdout(predstr::similar(expected).distance(2));
        Ok(())
    }

    #[test]
    fn regex() -> AnyResult {
        let root = TestDir::new(test_dir!())?;
        let bytes: Vec<_> = random_collection(4096);
        let particular_1_name = root.write_file(&"particular_1_name", &bytes)?;
        let particular_2_name = root.write_file(&"particular_2_name", &bytes)?;
        root.write_file(&"not_particular_2_name", &bytes)?;
        root.write_file(&"completely_different", &bytes)?;
        let expected = [
            particular_1_name.to_string_lossy(),
            particular_2_name.to_string_lossy(),
        ]
        .join("\n")
            + "\n";
        assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
            .args(&["--regex", "^particular_\\d_name$"])
            .arg(root.as_ref())
            .assert()
            .success()
            .stderr(predstr::is_empty())
            .stdout(predstr::similar(expected).distance(2));
        Ok(())
    }

    #[test]
    fn glob_pattern() -> AnyResult {
        let root = TestDir::new(test_dir!())?;
        let bytes: Vec<_> = random_collection(4096);
        let particular_1_name = root.write_file(&"particular_1_name", &bytes)?;
        let particular_2_name = root.write_file(&"particular_2_name", &bytes)?;
        root.write_file(&"not_particular_2_name", &bytes)?;
        root.write_file(&"completely_different", &bytes)?;
        let expected = [
            particular_1_name.to_string_lossy(),
            particular_2_name.to_string_lossy(),
        ]
        .join("\n")
            + "\n";
        assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
            .args(&["--pattern", "particular*name"])
            .arg(root.as_ref())
            .assert()
            .success()
            .stderr(predstr::is_empty())
            .stdout(predstr::similar(expected).distance(2));
        Ok(())
    }

    #[test]
    fn min_file_size() -> AnyResult {
        let root = TestDir::new(test_dir!())?;
        let bytes: Vec<_> = random_collection(4096);
        let particular_1_name = root.write_file(&"particular_1_name", &bytes)?;
        let particular_2_name = root.write_file(&"particular_2_name", &bytes)?;
        root.write_file(&"not_particular_2_name", &bytes[..2048])?;
        root.write_file(&"completely_different", &bytes[..2048])?;
        let expected = [
            particular_1_name.to_string_lossy(),
            particular_2_name.to_string_lossy(),
        ]
        .join("\n")
            + "\n";
        assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
            .args(&["--min", "4K"])
            .arg(root.as_ref())
            .assert()
            .success()
            .stderr(predstr::is_empty())
            .stdout(predstr::similar(expected).distance(2));
        Ok(())
    }

    #[test]
    fn max_file_size() -> AnyResult {
        let root = TestDir::new(test_dir!())?;
        let bytes: Vec<_> = random_collection(4096);
        let particular_1_name = root.write_file(&"particular_1_name", &bytes[..1024])?;
        let particular_2_name = root.write_file(&"particular_2_name", &bytes[..1024])?;
        root.write_file(&"not_particular_2_name", &bytes)?;
        root.write_file(&"completely_different", &bytes)?;
        let expected = [
            particular_1_name.to_string_lossy(),
            particular_2_name.to_string_lossy(),
        ]
        .join("\n")
            + "\n";
        assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
            .args(&["--max", "2K"])
            .arg(root.as_ref())
            .assert()
            .success()
            .stderr(predstr::is_empty())
            .stdout(predstr::similar(expected).distance(2));
        Ok(())
    }
}
