pub mod auth;
pub mod auth_fake;
pub mod db;
pub mod db_fake;
pub mod fs;
pub mod fs_fake;
pub mod git;
pub mod git_fake;
pub mod github;
pub mod github_fake;
pub mod test_support;

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

/// Builds real ports from environment variables.
/// Falls back to fakes when FAKE_AUTH=true or FAKE_DB=true.
pub async fn ports_from_env() -> Ports {
    let auth: Arc<dyn crate::ports::AuthPort> = if std::env::var("FAKE_AUTH").as_deref() == Ok("true") {
        Arc::new(auth_fake::FakeAuth::default())
    } else {
        Arc::new(auth::HydraAuth::faf())
    };

    let db: Arc<dyn crate::ports::DbPort> = if std::env::var("FAKE_DB").as_deref() == Ok("true") {
        Arc::new(db_fake::FakeDb::default())
    } else {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required when FAKE_DB != true");
        Arc::new(db::SqlxDb::connect(&url).await.expect("DB connection failed"))
    };

    Ports {
        auth,
        db,
        fs: Arc::new(fs::LocalFs::default()),
        git: Arc::new(git::GitInfra::default()),
        github: Arc::new(github::GithubClient::new()),
    }
}
