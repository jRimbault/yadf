use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub struct TestDir(PathBuf);

impl TestDir {
    pub fn new<P: AsRef<Path>>(dir: P) -> io::Result<Self> {
        match std::fs::remove_dir_all(&dir) {
            // the directory should not exists at this stage
            // we're just double checking
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
            _ => {}
        }
        std::fs::create_dir_all(&dir)?;
        Ok(TestDir(dir.as_ref().to_path_buf()))
    }

    pub fn write_file<P: AsRef<Path>, B: AsRef<[u8]>>(
        &self,
        path: &P,
        bytes: B,
    ) -> io::Result<PathBuf> {
        let path = self.0.join(path);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .open(&path)?;
        file.write_all(bytes.as_ref())?;
        Ok(path)
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("couldn't remove test directory");
    }
}

impl AsRef<Path> for TestDir {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}
