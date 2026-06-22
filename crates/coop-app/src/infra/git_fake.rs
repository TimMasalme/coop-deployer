use std::path::Path;

use async_trait::async_trait;
use crate::ports::git::{GitPort, GitResult};

/// Does nothing — simulates a successful checkout without touching the filesystem.
/// Tests that need actual files should seed them via FakeFs instead.
#[derive(Default)]
pub struct FakeGit;

#[async_trait]
impl GitPort for FakeGit {
    async fn checkout(&self, _url: &str, _git_ref: &str, _workdir: &Path) -> GitResult<()> {
        Ok(())
    }
}
