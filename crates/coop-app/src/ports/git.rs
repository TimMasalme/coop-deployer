use std::path::Path;

use async_trait::async_trait;
use coop_domain::errors::GitError;

pub type GitResult<T> = Result<T, GitError>;

#[async_trait]
pub trait GitPort: Send + Sync {
    async fn checkout(&self, url: &str, git_ref: &str, workdir: &Path) -> GitResult<()>;
}
