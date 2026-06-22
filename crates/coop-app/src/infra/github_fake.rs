use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use coop_domain::{errors::GithubError, models::Asset};

use crate::ports::github::{GithubPort, GithubResult};

/// Returns pre-seeded assets and file contents. No network. Use in tests.
#[derive(Default)]
pub struct FakeGithub {
    assets: Mutex<HashMap<String, Vec<Asset>>>,
    downloads: Mutex<HashMap<String, Vec<u8>>>,
}

impl FakeGithub {
    pub fn seed_assets(&self, repo: &str, tag: &str, assets: Vec<Asset>) {
        self.assets.lock().unwrap().insert(format!("{repo}@{tag}"), assets);
    }

    pub fn seed_download(&self, url: &str, content: impl Into<Vec<u8>>) {
        self.downloads.lock().unwrap().insert(url.to_string(), content.into());
    }
}

#[async_trait]
impl GithubPort for FakeGithub {
    async fn fetch_release_assets(&self, repo: &str, tag: &str) -> GithubResult<Vec<Asset>> {
        Ok(self
            .assets
            .lock()
            .unwrap()
            .get(&format!("{repo}@{tag}"))
            .cloned()
            .unwrap_or_default())
    }

    async fn download_asset(&self, url: &str) -> GithubResult<Vec<u8>> {
        self.downloads
            .lock()
            .unwrap()
            .get(url)
            .cloned()
            .ok_or_else(|| GithubError::new(format!("no seeded download for {url}")))
    }
}
