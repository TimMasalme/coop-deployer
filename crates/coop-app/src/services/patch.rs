use coop_domain::{errors::DeployError, models::{DeployResult, PatchRecord}};

use crate::ports::Ports;

pub struct PatchConfig {
    pub repo: String,
    pub tag: String,
    pub version: i32,
    pub dry_run: bool,
}

impl PatchConfig {
    pub fn from_env() -> Result<Self, DeployError> {
        Ok(Self {
            repo: std::env::var("COOP_PATCH_REPO")
                .unwrap_or_else(|_| "FAForever/fa-coop".into()),
            tag: std::env::var("PATCH_VERSION").unwrap_or_else(|_| "latest".into()),
            version: std::env::var("PATCH_VERSION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            dry_run: std::env::var("DRY_RUN").as_deref() == Ok("true"),
        })
    }
}

/// All 25 patch files, ported from CoopDeployer.kt.
struct PatchFile {
    file_id: i32,
    name: &'static str,
}

const PATCH_FILES: &[PatchFile] = &[
    PatchFile { file_id: 1,  name: "init_coop" },
    PatchFile { file_id: 2,  name: "lobby_coop" },
    PatchFile { file_id: 3,  name: "A01_VO" },
    PatchFile { file_id: 4,  name: "A02_VO" },
    PatchFile { file_id: 5,  name: "A03_VO" },
    PatchFile { file_id: 6,  name: "A04_VO" },
    PatchFile { file_id: 7,  name: "A05_VO" },
    PatchFile { file_id: 8,  name: "A06_VO" },
    PatchFile { file_id: 9,  name: "C01_VO" },
    PatchFile { file_id: 10, name: "C02_VO" },
    PatchFile { file_id: 11, name: "C03_VO" },
    PatchFile { file_id: 12, name: "C04_VO" },
    PatchFile { file_id: 13, name: "C05_VO" },
    PatchFile { file_id: 14, name: "C06_VO" },
    PatchFile { file_id: 15, name: "E01_VO" },
    PatchFile { file_id: 16, name: "E02_VO" },
    PatchFile { file_id: 17, name: "E03_VO" },
    PatchFile { file_id: 18, name: "E04_VO" },
    PatchFile { file_id: 19, name: "E05_VO" },
    PatchFile { file_id: 20, name: "E06_VO" },
    PatchFile { file_id: 21, name: "Prothyon16_VO" },
    PatchFile { file_id: 22, name: "TCR_VO" },
    PatchFile { file_id: 23, name: "SCCA_Briefings" },
    PatchFile { file_id: 24, name: "SCCA_FMV" },
    PatchFile { file_id: 25, name: "FAF_Coop_Operation_Tight_Spot_VO" },
];

pub async fn deploy_patches(ports: &Ports, config: &PatchConfig) -> Result<DeployResult, DeployError> {
    let assets = ports.github
        .fetch_release_assets(&config.repo, &config.tag)
        .await
        .map_err(|e| DeployError::new(format!("failed to fetch release assets: {e}")))?;

    let mut result = DeployResult::default();

    for patch in PATCH_FILES {
        let asset = assets.iter().find(|a| a.name.contains(patch.name));

        let content = match asset {
            Some(a) => ports.github
                .download_asset(&a.download_url)
                .await
                .map_err(|e| DeployError::new(format!("download failed for {}: {e}", patch.name)))?,
            None => {
                eprintln!("info: patch '{}' not in release {} (unchanged), skipping", patch.name, config.tag);
                result.skipped += 1;
                continue;
            }
        };

        let new_md5 = ports.fs.compute_md5(&content).await
            .map_err(|e| DeployError::new(format!("md5 failed: {e}")))?;

        let existing = ports.db.get_patch_record(patch.file_id).await
            .map_err(|e| DeployError::new(e.to_string()))?;

        if existing.as_ref().map(|r| r.md5 == new_md5).unwrap_or(false) {
            result.skipped += 1;
            continue;
        }

        if !config.dry_run {
            let next_version = existing.as_ref().map(|r| r.version + 1).unwrap_or(1);
            ports.db.upsert_patch(PatchRecord {
                file_id: patch.file_id,
                name: patch.name.to_string(),
                md5: new_md5,
                version: next_version,
            }).await.map_err(|e| DeployError::new(e.to_string()))?;
        }

        result.updated += 1;
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use coop_domain::models::Asset;

    use super::*;
    use crate::infra::test_support::TestPorts;
    use crate::ports::db::DbPort;

    fn test_config() -> PatchConfig {
        PatchConfig {
            repo: "FAForever/fa-coop".into(),
            tag: "v3".into(),
            version: 3,
            dry_run: false,
        }
    }

    #[tokio::test]
    async fn new_asset_is_deployed() {
        let tp = TestPorts::new();
        tp.github.seed_assets("FAForever/fa-coop", "v3", vec![
            Asset { name: "init_coop.nx2".into(), download_url: "http://fake/init_coop".into() },
        ]);
        tp.github.seed_download("http://fake/init_coop", b"patch content");

        let result = deploy_patches(&tp.ports(), &test_config()).await.unwrap();
        assert_eq!(result.updated, 1);
    }

    #[tokio::test]
    async fn unchanged_asset_is_skipped() {
        let tp = TestPorts::new();
        tp.github.seed_assets("FAForever/fa-coop", "v3", vec![
            Asset { name: "init_coop.nx2".into(), download_url: "http://fake/init_coop".into() },
        ]);
        tp.github.seed_download("http://fake/init_coop", b"same content");

        deploy_patches(&tp.ports(), &test_config()).await.unwrap();
        let result = deploy_patches(&tp.ports(), &test_config()).await.unwrap();

        assert_eq!(result.skipped, 25);
    }

    #[tokio::test]
    async fn dry_run_does_not_write_db() {
        let tp = TestPorts::new();
        tp.github.seed_assets("FAForever/fa-coop", "v3", vec![
            Asset { name: "init_coop.nx2".into(), download_url: "http://fake/init_coop".into() },
        ]);
        tp.github.seed_download("http://fake/init_coop", b"content");

        let mut config = test_config();
        config.dry_run = true;
        deploy_patches(&tp.ports(), &config).await.unwrap();

        let record: Option<coop_domain::models::PatchRecord> = tp.db.get_patch_record(1).await.unwrap();
        assert!(record.is_none());
    }
}
