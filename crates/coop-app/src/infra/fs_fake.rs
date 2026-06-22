use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use async_trait::async_trait;
use coop_domain::{errors::FsError, models::ZipEntry};

use crate::ports::fs::{FsPort, FsResult};

/// In-memory filesystem. No disk access. Use in tests.
#[derive(Default)]
pub struct FakeFs {
    files: Mutex<HashMap<PathBuf, Vec<u8>>>,
    zips: Mutex<HashMap<PathBuf, Vec<ZipEntry>>>,
}

impl FakeFs {
    pub fn seed_file(&self, path: impl Into<PathBuf>, content: impl Into<Vec<u8>>) {
        self.files.lock().unwrap().insert(path.into(), content.into());
    }

    pub fn written_zip(&self, path: &Path) -> Option<Vec<ZipEntry>> {
        self.zips.lock().unwrap().get(path).cloned()
    }
}

#[async_trait]
impl FsPort for FakeFs {
    async fn list_files(&self, dir: &Path) -> FsResult<Vec<PathBuf>> {
        let files = self.files.lock().unwrap();
        let mut result: Vec<PathBuf> = files
            .keys()
            .filter(|p| p.starts_with(dir))
            .cloned()
            .collect();
        result.sort();
        Ok(result)
    }

    async fn read_file(&self, path: &Path) -> FsResult<Vec<u8>> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| FsError::new(format!("file not found: {}", path.display())))
    }

    async fn write_zip(&self, entries: Vec<ZipEntry>, dest: &Path) -> FsResult<()> {
        self.zips.lock().unwrap().insert(dest.to_path_buf(), entries);
        Ok(())
    }

    async fn compute_md5(&self, content: &[u8]) -> FsResult<String> {
        // Deterministic fake: hex of sum of bytes, good enough for change detection tests.
        let sum: u64 = content.iter().map(|&b| b as u64).sum();
        Ok(format!("{sum:032x}"))
    }

    async fn copy_file(&self, src: &Path, dest: &Path) -> FsResult<()> {
        let content = self.read_file(src).await?;
        self.files.lock().unwrap().insert(dest.to_path_buf(), content);
        Ok(())
    }
}
