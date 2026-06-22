use async_trait::async_trait;
use coop_domain::{errors::GithubError, models::Asset};

pub type GithubResult<T> = Result<T, GithubError>;

#[async_trait]
pub trait GithubPort: Send + Sync {
    async fn fetch_release_assets(&self, repo: &str, tag: &str) -> GithubResult<Vec<Asset>>;
    async fn download_asset(&self, url: &str) -> GithubResult<Vec<u8>>;
}
