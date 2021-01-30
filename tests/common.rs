/// quick-n-dirty any result type alias
pub type AnyResult<T = (), E = Box<dyn std::error::Error>> = Result<T, E>;

pub const MAX_LEN: usize = 256 * 1024;

pub fn random_collection<T, I>(size: usize) -> I
where
    rand::distributions::Standard: rand::distributions::Distribution<T>,
    I: std::iter::FromIterator<T>,
{
    use rand::Rng;
    let mut rng = rand::thread_rng();
    std::iter::repeat_with(|| rng.gen()).take(size).collect()
}

/// test shortcut
pub fn find_dupes<P: AsRef<std::path::Path>>(path: &P) -> yadf::TreeBag<u64, yadf::path::Path> {
    yadf::Yadf::builder()
        .paths(&[path])
        .build()
        .scan::<seahash::SeaHasher>()
}

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
