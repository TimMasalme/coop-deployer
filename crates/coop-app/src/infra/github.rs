use async_trait::async_trait;
use coop_domain::{errors::GithubError, models::Asset};
use serde::Deserialize;

use crate::ports::github::{GithubPort, GithubResult};

pub struct GithubClient {
    http: reqwest::Client,
}

impl GithubClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .user_agent("faf-coop-deployer/1.0")
                .build()
                .expect("failed to build HTTP client"),
        }
    }
}

#[derive(Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

#[async_trait]
impl GithubPort for GithubClient {
    async fn fetch_release_assets(&self, repo: &str, tag: &str) -> GithubResult<Vec<Asset>> {
        let url = format!("https://api.github.com/repos/{repo}/releases/tags/{tag}");
        let resp = self.http.get(&url).send().await
            .map_err(|e| GithubError::new(format!("request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(GithubError::new(format!("GitHub API returned {}", resp.status())));
        }

        #[derive(Deserialize)]
        struct Release { assets: Vec<GithubAsset> }

        let release: Release = resp.json().await
            .map_err(|e| GithubError::new(format!("parse failed: {e}")))?;

        Ok(release.assets.into_iter().map(|a| Asset {
            name: a.name,
            download_url: a.browser_download_url,
        }).collect())
    }

    async fn download_asset(&self, url: &str) -> GithubResult<Vec<u8>> {
        let resp = self.http.get(url).send().await
            .map_err(|e| GithubError::new(format!("download failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(GithubError::new(format!("download returned {}", resp.status())));
        }

        resp.bytes().await
            .map(|b| b.to_vec())
            .map_err(|e| GithubError::new(format!("read body failed: {e}")))
    }
}
