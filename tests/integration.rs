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
    assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
        .arg("-vvvv") // test stderr contains enough debug output
        .args(&["--format", "json"])
        .args(&["--algorithm", "seahash"])
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
                .and(predstr::contains("file3"))
                .and(predstr::contains("file4")),
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
    assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
        .args(&["--regex", "^particular_\\d_name$"])
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
    assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
        .args(&["--pattern", "particular*name"])
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
    assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
        .args(&["--min", "4K"])
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
    assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
        .args(&["--max", "2K"])
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
    root.write_file(&"aa", b"")?;
    assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
        .arg(root.as_ref())
        .args(&["-f", "json"])
        .arg("-vv")
        .assert()
        .success();
    Ok(())
}

#[test]
fn hard_links_flag() -> AnyResult {
    let predicate = predstr::contains("--hard-links");
    #[cfg(not(unix))]
    let predicate = predicate.not();
    assert_cmd::Command::cargo_bin(assert_cmd::crate_name!())?
        .arg("-h")
        .assert()
        .success()
        .stdout(predicate);
    Ok(())
}
