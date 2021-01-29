mod common;
mod test_dir;

use common::{find_dupes, random_collection, AnyResult, MAX_LEN};
use test_dir::TestDir;

macro_rules! scope_name_iter {
    () => {{
        fn fxxfxxf() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        type_name_of(fxxfxxf)
            .split("::")
            .take_while(|&segment| segment != "fxxfxxf")
    }};
}

macro_rules! test_dir {
    () => {{
        ["target", "tests"]
            .iter()
            .copied()
            .chain(scope_name_iter!())
            .collect::<std::path::PathBuf>()
    }};
}

#[test]
fn identical_small_files() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    println!("{:?}", root.as_ref());
    root.write_file("file1", b"aaa")?;
    root.write_file("file2", b"aaa")?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 1);
    assert_eq!(counter.as_tree().len(), 1);
    Ok(())
}

#[test]
fn identical_larger_files() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    let buffer: Vec<_> = random_collection(MAX_LEN * 3);
    root.write_file("file1", &buffer)?;
    root.write_file("file2", &buffer)?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 1);
    assert_eq!(counter.as_tree().len(), 1);
    Ok(())
}

#[test]
fn files_differing_by_size() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    root.write_file("file1", b"aaaa")?;
    root.write_file("file2", b"aaa")?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_tree().len(), 2);
    Ok(())
}

#[test]
fn files_differing_by_prefix() -> AnyResult {
    let root = TestDir::new(test_dir!())?;
    root.write_file("file1", b"aaa")?;
    root.write_file("file2", b"bbb")?;
    let counter = find_dupes(&root);
    assert_eq!(counter.duplicates().iter().count(), 0);
    assert_eq!(counter.as_tree().len(), 2);
    Ok(())
}

#[test]
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
    assert_eq!(counter.as_tree().len(), 2);
    Ok(())
}

#[test]
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
    assert_eq!(counter.as_tree().len(), 2);
    Ok(())
}
