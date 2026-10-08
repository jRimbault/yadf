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

    let groups = |factor| groups(&root, factor);
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

/// The groups `factor` reports under `root`, as sorted lists of file names.
fn groups(root: &TestDir, factor: Factor) -> Vec<Vec<String>> {
    let counter = find_replicates(root, factor);
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
}

/// What the partial hash of a file shorter than a block reads: its size,
/// then its whole content.
fn size_prefixed(content: &[u8]) -> Vec<u8> {
    let mut bytes = (content.len() as u64).to_le_bytes().to_vec();
    bytes.extend_from_slice(content);
    bytes
}

/// `large` is, byte for byte, what the partial hash of `small` reads: a
/// hash of the whole of `large` must not land in `small`'s bucket.
#[test]
fn full_hash_never_matches_a_partial_hash() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let small = [b'A'; 4088];
    root.write_file("small", small)?;
    // shares small's size, so that small gets read and hashed
    root.write_file("other", [b'B'; 4088])?;
    let large = size_prefixed(&small);
    root.write_file("large1", &large)?;
    root.write_file("large2", &large)?;

    assert_eq!(
        groups(&root, Factor::Under(5)),
        [vec!["large1", "large2"], vec!["other"], vec!["small"]]
    );
    assert_eq!(
        groups(&root, Factor::Equal(1)),
        [vec!["other"], vec!["small"]]
    );
    Ok(())
}

/// `tail`'s last block is what the partial hash of `small` reads: a hash
/// of that block must not land in `small`'s bucket.
#[test]
fn suffix_hash_never_matches_a_partial_hash() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let small = [b'A'; 4088];
    root.write_file("small", small)?;
    root.write_file("other", [b'B'; 4088])?;
    // same size and head, so both get their tail hashed, which differs
    let head: Vec<u8> = random_collection(MAX_LEN);
    let mut tail = head.clone();
    tail.extend_from_slice(&size_prefixed(&small));
    let mut twin = head;
    twin.extend_from_slice(&[b'C'; 4096]);
    root.write_file("tail", &tail)?;
    root.write_file("twin", &twin)?;

    assert_eq!(
        groups(&root, Factor::Equal(1)),
        [vec!["other"], vec!["small"], vec!["tail"], vec!["twin"]]
    );
    Ok(())
}

/// Same size, and `head`'s first block is `tail`'s last one: hashing the
/// size along both blocks can't tell them apart, only the pass can.
#[test]
fn suffix_hash_never_matches_a_partial_hash_of_the_same_size() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let block: Vec<u8> = random_collection(4096);
    let body: Vec<u8> = random_collection(MAX_LEN);
    // alone with its first block, so its partial hash is final
    let mut head = block.clone();
    head.extend_from_slice(&body);
    // tail and twin share their first block, so both get their tail hashed
    let mut tail = body.clone();
    tail.extend_from_slice(&block);
    let mut twin = body;
    twin.extend_from_slice(&[b'C'; 4096]);
    root.write_file("head", &head)?;
    root.write_file("tail", &tail)?;
    root.write_file("twin", &twin)?;

    assert_eq!(
        groups(&root, Factor::Equal(1)),
        [vec!["head"], vec!["tail"], vec!["twin"]]
    );
    Ok(())
}

/// Each prefix bucket is split by tail on its own: two files left alone in
/// buckets of different prefixes but sharing a tail must not be merged back.
#[test]
fn suffix_singletons_from_different_prefix_buckets_stay_separate() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    const BLOCK: usize = 4096;
    const SIZE: usize = 128 * 1024;

    for (name, prefix, suffix) in [
        ("a1", b'A', b'X'),
        ("a2", b'A', b'Y'),
        ("b1", b'B', b'X'),
        ("b2", b'B', b'Z'),
    ] {
        let mut content = vec![b'M'; SIZE];
        content[..BLOCK].fill(prefix);
        content[SIZE - BLOCK..].fill(suffix);
        root.write_file(name, &content)?;
    }

    let expected = [vec!["a1"], vec!["a2"], vec!["b1"], vec!["b2"]];

    for factor in [
        Factor::Under(5),
        Factor::Equal(1),
        Factor::Under(2),
        Factor::Over(0),
    ] {
        assert_eq!(groups(&root, factor), expected, "factor: {factor:?}");
    }

    assert!(groups(&root, Factor::Over(1)).is_empty());
    Ok(())
}
