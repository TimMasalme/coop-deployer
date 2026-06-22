use std::sync::Arc;

use crate::{
    infra::{
        auth_fake::FakeAuth, db_fake::FakeDb, fs_fake::FakeFs,
        git_fake::FakeGit, github_fake::FakeGithub,
    },
    ports::Ports,
};

/// Holds concrete fake implementations so tests can call seed helpers directly,
/// then convert to a `Ports` bundle for passing to services.
pub struct TestPorts {
    pub auth: Arc<FakeAuth>,
    pub db: Arc<FakeDb>,
    pub fs: Arc<FakeFs>,
    pub git: Arc<FakeGit>,
    pub github: Arc<FakeGithub>,
}

impl TestPorts {
    pub fn new() -> Self {
        Self {
            auth: Arc::new(FakeAuth::default()),
            db: Arc::new(FakeDb::default()),
            fs: Arc::new(FakeFs::default()),
            git: Arc::new(FakeGit::default()),
            github: Arc::new(FakeGithub::default()),
        }
    }

    pub fn ports(&self) -> Ports {
        Ports {
            auth: self.auth.clone(),
            db: self.db.clone(),
            fs: self.fs.clone(),
            git: self.git.clone(),
            github: self.github.clone(),
        }
    }
}
