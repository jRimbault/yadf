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
        fs::remove_dir_all(&self.0).expect(&format!("couldn't remove test directory {:?}", self.0));
    }
}

impl AsRef<Path> for TestDir {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}
