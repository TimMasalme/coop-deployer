use std::path::Path;

use async_trait::async_trait;
use coop_domain::{errors::FsError, models::ZipEntry};

pub type FsResult<T> = Result<T, FsError>;

#[async_trait]
pub trait FsPort: Send + Sync {
    async fn list_files(&self, dir: &Path) -> FsResult<Vec<std::path::PathBuf>>;
    async fn read_file(&self, path: &Path) -> FsResult<Vec<u8>>;
    async fn write_zip(&self, entries: Vec<ZipEntry>, dest: &Path) -> FsResult<()>;
    async fn compute_md5(&self, content: &[u8]) -> FsResult<String>;
    async fn copy_file(&self, src: &Path, dest: &Path) -> FsResult<()>;
}
