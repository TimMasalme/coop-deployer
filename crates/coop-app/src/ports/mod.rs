pub mod auth;
pub mod db;
pub mod fs;
pub mod git;
pub mod github;

use std::sync::Arc;

pub use auth::AuthPort;
pub use db::DbPort;
pub use fs::FsPort;
pub use git::GitPort;
pub use github::GithubPort;

pub struct Ports {
    pub auth: Arc<dyn AuthPort>,
    pub db: Arc<dyn DbPort>,
    pub fs: Arc<dyn FsPort>,
    pub git: Arc<dyn GitPort>,
    pub github: Arc<dyn GithubPort>,
}
