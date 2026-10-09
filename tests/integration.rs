mod common;

use common::{random_collection, AnyResult, TestDir, MAX_LEN};
use predicates::{boolean::PredicateBooleanExt, str as predstr};

#[test]
fn function_name() {
    let fname = scope_name_iter!().collect::<Vec<_>>().join("::");
    assert_eq!(fname, "integration::function_name");
}

#[test]
fn dir_macro() {
    let path = test_dir!();
    #[cfg(windows)]
    assert_eq!(path.to_str(), Some("target\\tests\\integration\\dir_macro"));
    #[cfg(not(windows))]
    assert_eq!(path.to_str(), Some("target/tests/integration/dir_macro"));
}

#[test]
fn trace_output() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    println!("{:?}", root.as_ref());
    let bytes: Vec<_> = random_collection(MAX_LEN);
    let file1 = root.write_file("file1", &bytes)?;
    let file2 = root.write_file("file2", &bytes)?;
    root.write_file("file3", &bytes[..4096])?;
    root.write_file("file4", &bytes[..2048])?;
    let _expected = serde_json::to_string(&[[file1.to_string_lossy(), file2.to_string_lossy()]])
        .unwrap()
        + "\n";
    assert_cmd::Command::cargo_bin(assert_cmd::pkg_name!())?
        .arg("-vvvv") // test stderr contains enough debug output
        .args(["--format", "json"])
        .args(["--algorithm", "seahash"])
        .arg(root.as_ref())
        .assert()
        .success()
        .stderr(
            predstr::contains("Args {")
                .and(predstr::contains("Yadf {"))
                .and(predstr::contains("format: Json"))
                .and(predstr::contains("algorithm: SeaHash"))
                .and(predstr::contains("verbose: 4"))
                .and(predstr::contains(
                    "found 2 possible duplicates after initial scan",
                ))
                .and(predstr::contains(
                    "found 2 duplicates in 1 groups after checksumming",
                ))
                .and(predstr::contains("file1"))
                .and(predstr::contains("file2"))
                // Unique sizes are pruned before any hashing, so they
                // never reach the traced buckets.
                .and(predstr::contains("file3").not())
                .and(predstr::contains("file4").not()),
        );
    Ok(())
}

#[test]
fn regex() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let bytes: Vec<_> = random_collection(4096);
    let particular_1_name = root.write_file("particular_1_name", &bytes)?;
    let particular_2_name = root.write_file("particular_2_name", &bytes)?;
    root.write_file("not_particular_2_name", &bytes)?;
    root.write_file("completely_different", &bytes)?;
    let _expected = [
        particular_1_name.to_string_lossy(),
        particular_2_name.to_string_lossy(),
    ]
    .join("\n")
        + "\n";
    assert_cmd::Command::cargo_bin(assert_cmd::pkg_name!())?
        .args(["--regex", "^particular_\\d_name$"])
        .arg(root.as_ref())
        .assert()
        .success()
        .stderr(predstr::is_empty());
    Ok(())
}

#[test]
fn glob_pattern() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let bytes: Vec<_> = random_collection(4096);
    let particular_1_name = root.write_file("particular_1_name", &bytes)?;
    let particular_2_name = root.write_file("particular_2_name", &bytes)?;
    root.write_file("not_particular_2_name", &bytes)?;
    root.write_file("completely_different", &bytes)?;
    let _expected = [
        particular_1_name.to_string_lossy(),
        particular_2_name.to_string_lossy(),
    ]
    .join("\n")
        + "\n";
    assert_cmd::Command::cargo_bin(assert_cmd::pkg_name!())?
        .args(["--pattern", "particular*name"])
        .arg(root.as_ref())
        .assert()
        .success()
        .stderr(predstr::is_empty());
    Ok(())
}

#[test]
fn min_file_size() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let bytes: Vec<_> = random_collection(4096);
    let particular_1_name = root.write_file("particular_1_name", &bytes)?;
    let particular_2_name = root.write_file("particular_2_name", &bytes)?;
    root.write_file("not_particular_2_name", &bytes[..2048])?;
    root.write_file("completely_different", &bytes[..2048])?;
    let _expected = [
        particular_1_name.to_string_lossy(),
        particular_2_name.to_string_lossy(),
    ]
    .join("\n")
        + "\n";
    assert_cmd::Command::cargo_bin(assert_cmd::pkg_name!())?
        .args(["--min", "4K"])
        .arg(root.as_ref())
        .assert()
        .success()
        .stderr(predstr::is_empty());
    Ok(())
}

#[test]
fn max_file_size() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let bytes: Vec<_> = random_collection(4096);
    let particular_1_name = root.write_file("particular_1_name", &bytes[..1024])?;
    let particular_2_name = root.write_file("particular_2_name", &bytes[..1024])?;
    root.write_file("not_particular_2_name", &bytes)?;
    root.write_file("completely_different", &bytes)?;
    let _expected = [
        particular_1_name.to_string_lossy(),
        particular_2_name.to_string_lossy(),
    ]
    .join("\n")
        + "\n";
    assert_cmd::Command::cargo_bin(assert_cmd::pkg_name!())?
        .args(["--max", "2K"])
        .arg(root.as_ref())
        .assert()
        .success()
        .stderr(predstr::is_empty());
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn non_utf8_paths() -> AnyResult {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;
    use std::path::PathBuf;
    let root = TestDir::new(test_dir!())?;
    let filename = PathBuf::from(OsString::from_vec(b"\xe7\xe7".to_vec()));
    root.write_file(&filename, b"")?;
    root.write_file("aa", b"")?;
    assert_cmd::Command::cargo_bin(assert_cmd::pkg_name!())?
        .arg(root.as_ref())
        .args(["-f", "json"])
        .arg("-vv")
        .assert()
        .success();
    Ok(())
}

/// Two names that are not Unicode and differ only by that byte stay two
/// distinct paths, each recoverable to its exact bytes.
#[cfg(all(unix, not(target_os = "macos")))]
#[test]
fn non_utf8_paths_are_lossless() -> AnyResult {
    use std::ffi::OsString;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    use std::path::PathBuf;
    let root = TestDir::new(test_dir!())?;
    let names = [b"\xff".as_slice(), b"\xfe".as_slice()];
    let paths = names.map(|name| {
        let name = PathBuf::from(OsString::from_vec(name.to_vec()));
        root.write_file(name, b"same")
    });
    let [first, second] = paths;
    let mut expected = [first?, second?].map(|path| path.as_os_str().as_bytes().to_vec());
    expected.sort();
    let yadf = |format: &str| -> AnyResult<Vec<u8>> {
        let output = assert_cmd::Command::cargo_bin(assert_cmd::pkg_name!())?
            .args(["-f", format])
            .arg(root.as_ref())
            .output()?;
        assert!(output.status.success(), "{format}: {output:?}");
        Ok(output.stdout)
    };

    // The order of the files within a group is not stable, compare them sorted.
    let sorted = |mut paths: Vec<Vec<u8>>| {
        paths.sort();
        paths
    };

    let json: Vec<Vec<serde_json::Value>> = serde_json::from_slice(&yadf("json")?)?;
    let [group] = <[_; 1]>::try_from(json).expect("one group");
    let decode = |path: serde_json::Value| -> Vec<u8> {
        use base64::Engine;
        let bytes = path["bytes"]
            .as_str()
            .expect("a non-Unicode path is an object");
        base64::engine::general_purpose::STANDARD
            .decode(bytes)
            .unwrap()
    };
    assert_eq!(sorted(group.into_iter().map(decode).collect()), expected);

    let csv = yadf("csv")?;
    let record = csv
        .strip_prefix(b"count,files\n2,")
        .and_then(|rest| rest.strip_suffix(b"\n"))
        .expect("a header and one group of two");
    let fields = record.split(|&byte| byte == b',').map(<[u8]>::to_vec);
    assert_eq!(sorted(fields.collect()), expected);

    let machine = yadf("machine")?;
    let group = machine
        .strip_suffix(b"\0\0")
        .expect("one group, ended by an empty record");
    let paths = group.split(|&byte| byte == 0).map(<[u8]>::to_vec);
    assert_eq!(sorted(paths.collect()), expected);

    let fdupes = yadf("fdupes")?;
    let group = fdupes
        .strip_suffix(b"\n\n")
        .expect("one group, ended by a blank line");
    let paths = group.split(|&byte| byte == b'\n').map(<[u8]>::to_vec);
    assert_eq!(sorted(paths.collect()), expected);
    Ok(())
}

#[test]
fn hard_links_flag() -> AnyResult {
    let predicate = predstr::contains("--hard-links");
    #[cfg(not(unix))]
    let predicate = predicate.not();
    assert_cmd::Command::cargo_bin(assert_cmd::pkg_name!())?
        .arg("-h")
        .assert()
        .success()
        .stdout(predicate);
    Ok(())
}

/// Regression test for issue #8: on a single-CPU machine every rayon pool has
/// one worker, and the scan used to deadlock before reading a single file.
/// `RAYON_NUM_THREADS` sizes the global pool the walk runs on, `--io-threads`
/// the hashing pool; the timeout turns a deadlock into a failure.
#[test]
fn single_thread_does_not_deadlock() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let bytes: Vec<_> = random_collection(MAX_LEN * 3);
    let file1 = root.write_file("file1", &bytes)?;
    let file2 = root.write_file("file2", &bytes)?;
    root.write_file("unique", &bytes[..MAX_LEN])?;
    assert_cmd::Command::cargo_bin(assert_cmd::pkg_name!())?
        .env("RAYON_NUM_THREADS", "1")
        .args(["--io-threads", "1", "--format", "machine"])
        .arg(root.as_ref())
        .timeout(std::time::Duration::from_secs(30))
        .assert()
        .success()
        .stdout(
            // The machine format ends each path with a NUL.
            predstr::contains(format!("{}\0", file1.display()))
                .and(predstr::contains(format!("{}\0", file2.display())))
                .and(predstr::contains("unique").not()),
        );
    Ok(())
}
