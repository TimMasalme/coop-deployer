use std::path::Path;

use async_trait::async_trait;
use coop_domain::errors::GitError;

use crate::ports::git::{GitPort, GitResult};

#[derive(Default)]
pub struct GitInfra;

#[async_trait]
impl GitPort for GitInfra {
    async fn checkout(&self, url: &str, git_ref: &str, workdir: &Path) -> GitResult<()> {
        let url = url.to_string();
        let git_ref = git_ref.to_string();
        let workdir = workdir.to_path_buf();

        tokio::task::spawn_blocking(move || {
            if workdir.exists() {
                // Pull latest if already cloned.
                let repo = git2::Repository::open(&workdir)
                    .map_err(|e| GitError::new(format!("open repo failed: {e}")))?;
                let mut remote = repo.find_remote("origin")
                    .map_err(|e| GitError::new(format!("find remote failed: {e}")))?;
                remote.fetch(&[&git_ref], None, None)
                    .map_err(|e| GitError::new(format!("fetch failed: {e}")))?;
            } else {
                git2::Repository::clone(&url, &workdir)
                    .map_err(|e| GitError::new(format!("clone failed: {e}")))?;
            }
            Ok(())
        })
        .await
        .map_err(|e| GitError::new(format!("task failed: {e}")))?
    }
}
