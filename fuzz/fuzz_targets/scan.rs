#![no_main]

use libfuzzer_sys::fuzz_target;
use std::collections::BTreeSet;
use std::path::PathBuf;

const MAX_FILES: usize = 4;
const FIELDS_PER_FILE: usize = 6;

#[derive(Default)]
struct Digest(blake3::Hasher);

impl yadf::Hasher for Digest {
    type Hash = [u8; 32];

    fn write(&mut self, bytes: &[u8]) {
        self.0.update(bytes);
    }

    fn finish(self) -> Self::Hash {
        self.0.finalize().into()
    }
}

fn contents(fields: &[u8]) -> Vec<u8> {
    let len = match fields[0] % 9 {
        0 => 0,
        1 => 1,
        2 => 4095,
        3 => 4096,
        4 => 4097,
        5 => 65535,
        6 => 65536,
        7 => 65537,
        _ => usize::from(fields[1]) * 32,
    };
    let mut bytes = vec![fields[1]; len];
    if len > 0 {
        bytes[0] = fields[2];
        bytes[len / 2] = fields[3];
        bytes[len - 1] = fields[4];
    }
    bytes
}

fn check(data: &[u8]) {
    let Some((&first, rest)) = data.split_first() else {
        return;
    };
    let count = usize::from(first) % MAX_FILES + 1;
    if rest.len() < count * FIELDS_PER_FILE {
        return;
    }

    let directory = tempfile::tempdir().unwrap();
    let mut paths = Vec::<PathBuf>::with_capacity(count);
    let mut files = Vec::<Vec<u8>>::with_capacity(count);
    for (index, fields) in rest.chunks_exact(FIELDS_PER_FILE).take(count).enumerate() {
        let source = usize::from(fields[5]) % (index + 1);
        let bytes = if source < index {
            files[source].clone()
        } else {
            contents(fields)
        };
        let path = directory.path().join(format!("file-{index}"));
        std::fs::write(&path, &bytes).unwrap();
        paths.push(path);
        files.push(bytes);
    }

    let result = yadf::Yadf::builder()
        .paths([directory.path()].as_ref())
        .io_threads(2)
        .build()
        .scan::<Digest>();

    let mut seen = BTreeSet::new();
    let mut actual = BTreeSet::new();
    for bucket in result.as_inner().values() {
        for path in bucket {
            let index = paths
                .iter()
                .position(|expected| expected == path.as_ref())
                .expect("scan returned an unknown path");
            assert!(seen.insert(index), "scan returned a file twice");
        }
        for (position, left) in bucket.iter().enumerate() {
            for right in bucket.iter().skip(position + 1) {
                let left = paths.iter().position(|path| path == left.as_ref()).unwrap();
                let right = paths
                    .iter()
                    .position(|path| path == right.as_ref())
                    .unwrap();
                actual.insert((left.min(right), left.max(right)));
            }
        }
    }
    assert_eq!(seen.len(), count, "scan omitted a file");

    let expected: BTreeSet<_> = (0..count)
        .flat_map(|left| (left + 1..count).map(move |right| (left, right)))
        .filter(|&(left, right)| files[left] == files[right])
        .collect();
    assert_eq!(
        actual, expected,
        "duplicate groups differ from file contents"
    );
}

fuzz_target!(|data: &[u8]| check(data));
