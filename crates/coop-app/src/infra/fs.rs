use std::io::Write;
use std::path::{Path, PathBuf};

use async_trait::async_trait;
use coop_domain::{errors::FsError, models::ZipEntry};

use crate::ports::fs::{FsPort, FsResult};

#[derive(Default)]
pub struct LocalFs;

#[async_trait]
impl FsPort for LocalFs {
    async fn list_files(&self, dir: &Path) -> FsResult<Vec<PathBuf>> {
        let mut entries = vec![];
        let read = std::fs::read_dir(dir)
            .map_err(|e| FsError::new(format!("cannot read dir {}: {e}", dir.display())))?;
        for entry in read.flatten() {
            let path = entry.path();
            if path.is_file() {
                entries.push(path);
            }
        }
        entries.sort();
        Ok(entries)
    }

    async fn read_file(&self, path: &Path) -> FsResult<Vec<u8>> {
        std::fs::read(path)
            .map_err(|e| FsError::new(format!("cannot read {}: {e}", path.display())))
    }

    async fn write_zip(&self, entries: Vec<ZipEntry>, dest: &Path) -> FsResult<()> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| FsError::new(format!("cannot create dir: {e}")))?;
        }

        let file = std::fs::File::create(dest)
            .map_err(|e| FsError::new(format!("cannot create zip {}: {e}", dest.display())))?;
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        for entry in entries {
            let path_str = entry.path.to_string_lossy();
            zip.start_file(path_str.as_ref(), options)
                .map_err(|e| FsError::new(format!("zip entry error: {e}")))?;
            zip.write_all(&entry.content)
                .map_err(|e| FsError::new(format!("zip write error: {e}")))?;
        }

        zip.finish()
            .map_err(|e| FsError::new(format!("zip finish error: {e}")))?;
        Ok(())
    }

    async fn compute_md5(&self, content: &[u8]) -> FsResult<String> {
        let digest = md5::compute(content);
        Ok(format!("{digest:x}"))
    }

    async fn copy_file(&self, src: &Path, dest: &Path) -> FsResult<()> {
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| FsError::new(format!("cannot create dir: {e}")))?;
        }
        std::fs::copy(src, dest)
            .map(|_| ())
            .map_err(|e| FsError::new(format!("copy failed: {e}")))
    }
}
