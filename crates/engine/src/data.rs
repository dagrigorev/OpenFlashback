//! Locating and reading files in the user's Flashback data directory.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The directory holding the original data files (`level1.map`, `perso.spr`, ...).
///
/// File names are matched case-insensitively, since copies from DOS disks are often upper case.
#[derive(Debug, Clone)]
pub struct DataDir {
    root: PathBuf,
}

impl DataDir {
    /// Opens `root`, or its `data` subdirectory when that is where the files are.
    pub fn open(root: impl AsRef<Path>) -> io::Result<Self> {
        let root = root.as_ref();
        for dir in [root.to_path_buf(), root.join("data"), root.join("DATA")] {
            let d = DataDir { root: dir };
            if d.find("perso.spr").is_some() {
                return Ok(d);
            }
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("no Flashback data (perso.spr) in {}", root.display()),
        ))
    }

    /// The directory the files are read from.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path of `name` in the data directory, ignoring case.
    pub fn find(&self, name: &str) -> Option<PathBuf> {
        let direct = self.root.join(name);
        if direct.is_file() {
            return Some(direct);
        }
        fs::read_dir(&self.root)
            .ok()?
            .flatten()
            .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
            .map(|e| e.path())
    }

    /// Reads the whole file `name`.
    pub fn read(&self, name: &str) -> io::Result<Vec<u8>> {
        let path = self
            .find(name)
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, format!("missing data file {name}")))?;
        fs::read(path)
    }
}
