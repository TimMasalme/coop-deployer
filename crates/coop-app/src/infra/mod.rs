pub mod auth_fake;
pub mod db_fake;
pub mod fs_fake;
pub mod git_fake;
pub mod github_fake;

use std::sync::Arc;

use crate::ports::Ports;

pub fn fake_ports() -> Ports {
    Ports {
        auth: Arc::new(auth_fake::FakeAuth::default()),
        db: Arc::new(db_fake::FakeDb::default()),
        fs: Arc::new(fs_fake::FakeFs::default()),
        git: Arc::new(git_fake::FakeGit::default()),
        github: Arc::new(github_fake::FakeGithub::default()),
    }
}
