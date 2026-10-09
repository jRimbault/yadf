//! End-to-end tests of the CLI: every scan goes through the binary and its
//! JSON output, and the groups are checked against what the fixtures hold.
//!
//! The permission test is opt-in because it requires an unprivileged user.
#![cfg(feature = "build-bin")]

#[allow(dead_code)] // Other targets use additional shared helpers.
mod common;

use assert_cmd::{cargo::cargo_bin_cmd, Command};
use common::{AnyResult, TestDir};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

type Groups = Vec<Vec<PathBuf>>;
type Corpus = Vec<(PathBuf, Vec<u8>)>;

// Ignore presentation order, but NEVER remove repeated paths or groups.
fn normalize(mut groups: Groups) -> Groups {
    for group in &mut groups {
        group.sort();
    }
    groups.sort();
    groups
}

fn parse_groups(bytes: &[u8]) -> AnyResult<Groups> {
    let groups: Groups = serde_json::from_slice(bytes)?;
    Ok(normalize(groups))
}

// An independent oracle: compare complete fixture bytes, not yadf hashes.
fn oracle(corpus: &Corpus, keep: impl Fn(usize) -> bool) -> Groups {
    let mut by_content: BTreeMap<&[u8], Vec<PathBuf>> = BTreeMap::new();
    for (path, bytes) in corpus {
        by_content
            .entry(bytes.as_slice())
            .or_default()
            .push(path.clone());
    }
    normalize(
        by_content
            .into_values()
            .filter(|group| keep(group.len()))
            .collect(),
    )
}

fn command(threads: usize) -> Command {
    let mut cmd = cargo_bin_cmd!("yadf");
    cmd.env("RAYON_NUM_THREADS", threads.to_string())
        .arg("--io-threads")
        .arg(threads.to_string())
        .timeout(Duration::from_secs(30));
    cmd
}

fn run_json(cmd: &mut Command) -> AnyResult<Groups> {
    let checked = cmd.args(["--format", "json"]).assert().success().stderr("");
    parse_groups(&checked.get_output().stdout)
}

fn scan(root: &Path, args: &[&str]) -> AnyResult<Groups> {
    run_json(command(2).args(args).arg(root))
}

fn write_group(
    root: &TestDir,
    prefix: &str,
    copies: usize,
    bytes: &[u8],
) -> AnyResult<Vec<PathBuf>> {
    (0..copies)
        .map(|i| Ok(root.write_file(format!("{prefix}-{i}"), bytes)?))
        .collect()
}

#[test]
fn name_filters_assert_the_complete_result() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let a = root.write_file("particular_1_name", b"same contents")?;
    let b = root.write_file("particular_2_name", b"same contents")?;
    root.write_file("not_particular_2_name", b"same contents")?;
    root.write_file("completely_different", b"same contents")?;
    let expected = normalize(vec![vec![a, b]]);

    for args in [
        ["--regex", r"^particular_\d_name$"],
        ["--pattern", "particular*name"],
    ] {
        assert_eq!(scan(root.as_ref(), &args)?, expected, "{args:?}");
    }
    Ok(())
}

#[test]
fn size_limits_are_inclusive() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let below = write_group(&root, "below", 2, &vec![1; 4095])?;
    let at = write_group(&root, "at", 2, &vec![1; 4096])?;
    let above = write_group(&root, "above", 2, &vec![1; 4097])?;

    assert_eq!(
        scan(root.as_ref(), &["--min", "4096"])?,
        normalize(vec![at.clone(), above]),
    );
    assert_eq!(
        scan(root.as_ref(), &["--max", "4096"])?,
        normalize(vec![below, at.clone()]),
    );
    assert_eq!(
        scan(root.as_ref(), &["--min", "4096", "--max", "4096"])?,
        normalize(vec![at]),
    );
    Ok(())
}

#[test]
fn empty_files_are_reported_unless_excluded() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let empty = write_group(&root, "empty", 2, b"")?;
    let nonempty = write_group(&root, "nonempty", 2, b"x")?;

    assert_eq!(
        scan(root.as_ref(), &[])?,
        normalize(vec![empty, nonempty.clone()]),
    );
    assert_eq!(
        scan(root.as_ref(), &["--no-empty"])?,
        normalize(vec![nonempty]),
    );
    Ok(())
}

#[test]
fn matching_heads_and_tails_are_not_sufficient() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let mut bytes = vec![b'x'; 128 * 1024];
    let first = write_group(&root, "first", 2, &bytes)?;
    bytes[64 * 1024] = b'y';
    let second = write_group(&root, "second", 2, &bytes)?;
    bytes[64 * 1024] = b'z';
    root.write_file("unique-middle", &bytes)?;

    // All five files have the same size, first 4 KiB, and last 4 KiB.
    // Require two separate duplicate groups, not a single group of five.
    assert_eq!(scan(root.as_ref(), &[])?, normalize(vec![first, second]),);
    Ok(())
}

// These sizes straddle the current 4 KiB prefix, 64 KiB suffix-pass,
// and 256 KiB full-read buffer boundaries. Larger fixtures have matching
// heads/tails and differ only in their interiors.
fn boundary_corpus(root: &TestDir) -> AnyResult<Corpus> {
    let sizes = [
        0,
        1,
        4095,
        4096,
        4097,
        65535,
        65536,
        65537,
        262143,
        262144,
        262145,
        1024 * 1024 + 1,
    ];
    let mut corpus = Vec::new();
    for size in sizes {
        let base = vec![0x51; size];
        for copy in 0..2 {
            let path = root.write_file(format!("n{size}-base-{copy}"), &base)?;
            corpus.push((path, base.clone()));
        }
        if size != 0 {
            let mut changed = base;
            // Just past the prefix for medium files; past the first
            // full-read buffer for files larger than 256 KiB.
            let offset = if size > 262144 {
                262144
            } else if size > 4096 {
                4096
            } else {
                size - 1
            };
            changed[offset] ^= 1;
            for copy in 0..2 {
                let path = root.write_file(format!("n{size}-changed-{copy}"), &changed)?;
                corpus.push((path, changed.clone()));
            }
        }
    }
    Ok(corpus)
}

#[test]
fn all_algorithms_and_thread_counts_match_the_byte_oracle() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let corpus = boundary_corpus(&root)?;
    let expected = oracle(&corpus, |copies| copies > 1);

    for algorithm in [
        "ahash",
        "blake3",
        "highway",
        "metrohash",
        "seahash",
        "xxhash",
    ] {
        for threads in [1, 4] {
            let actual = run_json(
                command(threads)
                    .args(["--algorithm", algorithm])
                    .arg(root.as_ref()),
            )?;
            assert_eq!(actual, expected, "algorithm={algorithm}, threads={threads}");
        }
    }
    Ok(())
}

#[test]
fn many_candidates_finish_with_one_hash_worker() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    // More candidates than the current content-prefetch window (64).
    let paths = write_group(&root, "copy", 96, &vec![b'x'; 8192])?;
    assert_eq!(
        run_json(command(1).arg(root.as_ref()))?,
        normalize(vec![paths]),
    );
    Ok(())
}

#[test]
fn replication_factor_is_applied_after_content_splits() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let mut bytes = vec![b'x'; 128 * 1024];
    let pair = write_group(&root, "pair", 2, &bytes)?;
    bytes[8192] = b'y';
    let triple = write_group(&root, "triple", 3, &bytes)?;
    bytes[8192] = b'z';
    let singleton = write_group(&root, "singleton", 1, &bytes)?;
    let unique_size = write_group(&root, "unique-size", 1, b"short")?;

    // The six large files initially share size, prefix, and suffix.
    // Only full-content hashing separates their 2/3/1 groups.
    let cases = [
        ("equal:1", vec![singleton.clone(), unique_size.clone()]),
        ("equal:2", vec![pair.clone()]),
        ("equal:3", vec![triple.clone()]),
        ("equal:4", vec![]),
        ("under:3", vec![pair, singleton, unique_size]),
        ("over:2", vec![triple]),
    ];
    for (factor, expected) in cases {
        assert_eq!(
            scan(root.as_ref(), &["--rfactor", factor])?,
            normalize(expected),
            "factor={factor}",
        );
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn hardlink_policy_counts_file_identities_not_arbitrary_names() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let original = root.write_file("original", b"identical")?;
    let alias = root.as_ref().join("alias");
    std::fs::hard_link(&original, &alias)?;

    // Two names for one inode are not duplicates by default.
    assert!(scan(root.as_ref(), &[])?.is_empty());
    assert_eq!(
        scan(root.as_ref(), &["--hard-links"])?,
        normalize(vec![vec![original.clone(), alias.clone()]]),
    );

    let copy = root.write_file("independent-copy", b"identical")?;
    let actual = scan(root.as_ref(), &[])?;
    assert_eq!(actual.len(), 1);
    assert_eq!(actual[0].len(), 2);
    assert!(actual[0].contains(&copy));
    // Parallel traversal may retain either hardlink name. Do not require
    // one specific representative, but require exactly one of them.
    assert_ne!(actual[0].contains(&original), actual[0].contains(&alias));

    assert_eq!(
        scan(root.as_ref(), &["--hard-links"])?,
        normalize(vec![vec![original, alias, copy]]),
    );
    Ok(())
}

#[test]
fn repeated_equivalent_roots_do_not_duplicate_results() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let expected = normalize(vec![write_group(&root, "file", 2, b"same")?]);
    let mut cmd = command(2);
    // On Unix, disable inode filtering so it cannot hide repeated roots.
    #[cfg(unix)]
    cmd.arg("--hard-links");
    cmd.arg(root.as_ref()).arg(root.as_ref().join("."));
    assert_eq!(run_json(&mut cmd)?, expected);
    Ok(())
}

#[cfg(unix)]
#[test]
fn json_round_trips_special_utf8_filenames() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let mut paths = Vec::new();
    for name in [
        "space name",
        "line\nbreak",
        "tab\tname",
        "quote\"name",
        "back\\slash",
    ] {
        paths.push(root.write_file(name, b"identical")?);
    }
    assert_eq!(scan(root.as_ref(), &[])?, normalize(vec![paths]));
    Ok(())
}

#[test]
fn output_file_contains_the_report_and_stdout_is_empty() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let input = root.as_ref().join("input");
    std::fs::create_dir(&input)?;
    let a = root.write_file("input/a", b"same")?;
    let b = root.write_file("input/b", b"same")?;
    // Keep output outside the scan tree.
    let report = root.as_ref().join("report.json");
    command(2)
        .args(["--format", "json", "--output"])
        .arg(&report)
        .arg(&input)
        .assert()
        .success()
        .stdout("")
        .stderr("");
    assert_eq!(
        parse_groups(&std::fs::read(report)?)?,
        normalize(vec![vec![a, b]]),
    );
    Ok(())
}

#[test]
fn stdin_roots_are_used_instead_of_the_working_directory() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    // Canonical paths remain valid after changing the child's cwd.
    let absolute = dunce::canonicalize(root.as_ref())?;
    let first = absolute.join("first");
    let second = absolute.join("second");
    let fallback = absolute.join("empty-cwd");
    for dir in [&first, &second, &fallback] {
        std::fs::create_dir(dir)?;
    }
    let a = first.join("a");
    let b = second.join("b");
    std::fs::write(&a, b"same")?;
    std::fs::write(&b, b"same")?;
    // A line-based input format requires representable, newline-free roots.
    let first_text = first.to_str().ok_or("test root is not UTF-8")?;
    let second_text = second.to_str().ok_or("test root is not UTF-8")?;
    assert!(!first_text.contains('\n') && !first_text.contains('\r'));
    assert!(!second_text.contains('\n') && !second_text.contains('\r'));
    let actual = run_json(
        command(2)
            .current_dir(&fallback)
            .write_stdin(format!("{first_text}\n{second_text}\n")),
    )?;
    assert_eq!(actual, normalize(vec![vec![a, b]]));
    Ok(())
}

#[cfg(unix)]
#[test]
#[ignore = "requires an unprivileged user and Unix permission enforcement"]
fn unreadable_candidate_is_reported_and_excluded() -> AnyResult {
    use std::io::ErrorKind;
    use std::os::unix::fs::PermissionsExt;

    let root = TestDir::new(test_dir!())?;
    let bytes = vec![b'x'; 128 * 1024];
    let readable = write_group(&root, "readable", 2, &bytes)?;
    let blocked = root.write_file("blocked", &bytes)?;
    std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o000))?;

    // Fail rather than silently "pass" when root/capabilities bypass mode
    // bits. Only a regular file was chmod'ed; TestDir can still unlink it.
    let error = std::fs::File::open(&blocked)
        .expect_err("run this test without root or DAC-bypass capabilities");
    assert_eq!(error.kind(), ErrorKind::PermissionDenied);

    // This preserves the current best-effort scan policy. A future strict
    // mode should have its own test asserting a nonzero exit status.
    let checked = command(2)
        .args(["--format", "json"])
        .arg(root.as_ref())
        .assert()
        .success();
    let output = checked.get_output();
    assert_eq!(parse_groups(&output.stdout)?, normalize(vec![readable]));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("blocked"), "missing diagnostic: {stderr}");
    assert!(
        stderr.contains("couldn't hash"),
        "wrong diagnostic: {stderr}"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn disk_full_is_an_error() -> AnyResult {
    use std::os::unix::fs::FileTypeExt;

    // Check the fixture before letting yadf call File::create on this path.
    // In particular, never accidentally create a regular /dev/full file.
    assert!(std::fs::metadata("/dev/full")?.file_type().is_char_device());
    let root = TestDir::new(test_dir!())?;
    write_group(&root, "file", 2, b"same")?;

    // Tiny output stays in the 64 KiB BufWriter until its final flush, and
    // dropping the writer instead would swallow the error.
    for format in ["csv", "fdupes", "json", "json-pretty", "ld-json", "machine"] {
        command(2)
            .args(["--format", format, "--output", "/dev/full"])
            .arg(root.as_ref())
            .assert()
            .failure()
            .stdout("")
            .stderr(predicates::str::contains("writing output"));
    }
    Ok(())
}
