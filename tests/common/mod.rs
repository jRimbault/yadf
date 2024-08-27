pub use test_dir::TestDir;

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
#[allow(dead_code)]
pub fn find_dupes<P: AsRef<std::path::Path>>(path: &P) -> yadf::FileCounter {
    yadf::Yadf::builder()
        .paths(vec![path.as_ref().to_owned()])
        .build()
        .scan::<std::collections::hash_map::DefaultHasher>()
}

#[macro_export]
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

#[macro_export]
macro_rules! test_dir {
    () => {{
        ["target", "tests"]
            .iter()
            .copied()
            .chain(scope_name_iter!())
            .collect::<std::path::PathBuf>()
    }};
}

mod test_dir {
    use std::fs::{self, File};
    use std::io::{self, Write};
    use std::path::{Path, PathBuf};

    pub struct TestDir(PathBuf);

    impl TestDir {
        pub fn new<P>(dir: P) -> io::Result<Self>
        where
            P: AsRef<Path>,
        {
            match fs::remove_dir_all(&dir) {
                // the directory should not exists at this stage
                // we're just double checking and don't want to return a spurious error
                Err(e) if e.kind() == io::ErrorKind::NotFound => {}
                Err(e) => return Err(e),
                _ => {}
            }
            fs::create_dir_all(&dir)?;
            Ok(TestDir(dir.as_ref().to_path_buf()))
        }

        pub fn write_file<P, B>(&self, path: P, bytes: B) -> io::Result<PathBuf>
        where
            P: AsRef<Path>,
            B: AsRef<[u8]>,
        {
            let path = self.0.join(path);
            File::create(&path)?.write_all(bytes.as_ref())?;
            Ok(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0)
                .expect(&format!("couldn't remove test directory {:?}", self.0));
        }
    }

    impl AsRef<Path> for TestDir {
        fn as_ref(&self) -> &Path {
            &self.0
        }
    }
}
