use std::path::{Path, PathBuf};

use coop_domain::{errors::DeployError, models::{DeployResult, ZipEntry}};

use crate::ports::Ports;

pub struct MapConfig {
    pub repo_url: String,
    pub git_ref: String,
    pub workdir: PathBuf,
    pub map_dir: PathBuf,
    pub dry_run: bool,
}

impl MapConfig {
    pub fn from_env() -> Self {
        Self {
            repo_url: std::env::var("COOP_MAP_REPO")
                .unwrap_or_else(|_| "https://github.com/FAForever/faf-coop-maps".into()),
            git_ref: std::env::var("COOP_MAP_REF").unwrap_or_else(|_| "master".into()),
            workdir: PathBuf::from(std::env::var("GIT_WORKDIR").unwrap_or_else(|_| "/tmp/coop-maps".into())),
            map_dir: PathBuf::from(std::env::var("MAP_DIR").unwrap_or_else(|_| "/maps".into())),
            dry_run: std::env::var("DRY_RUN").as_deref() == Ok("true"),
        }
    }
}

/// All 31 co-op maps known to FAF, matching the IDs in the database.
/// Ported from CoopMapDeployer.kt.
pub struct KnownMap {
    pub id: i32,
    pub folder_name: &'static str,
}

pub const KNOWN_MAPS: &[KnownMap] = &[
    // UEF (X1CA)
    KnownMap { id: 1,  folder_name: "X1CA_Coop_001" },
    KnownMap { id: 3,  folder_name: "X1CA_Coop_002" },
    KnownMap { id: 4,  folder_name: "X1CA_Coop_003" },
    KnownMap { id: 5,  folder_name: "X1CA_Coop_004" },
    KnownMap { id: 6,  folder_name: "X1CA_Coop_005" },
    KnownMap { id: 7,  folder_name: "X1CA_Coop_006" },
    // Aeon (SCCA A)
    KnownMap { id: 8,  folder_name: "SCCA_Coop_A01" },
    KnownMap { id: 9,  folder_name: "SCCA_Coop_A02" },
    KnownMap { id: 10, folder_name: "SCCA_Coop_A03" },
    KnownMap { id: 11, folder_name: "SCCA_Coop_A04" },
    KnownMap { id: 12, folder_name: "SCCA_Coop_A05" },
    KnownMap { id: 13, folder_name: "SCCA_Coop_A06" },
    // Seraphim (SCCA E)
    KnownMap { id: 14, folder_name: "SCCA_Coop_E01" },
    KnownMap { id: 15, folder_name: "SCCA_Coop_E02" },
    KnownMap { id: 16, folder_name: "SCCA_Coop_E03" },
    KnownMap { id: 17, folder_name: "SCCA_Coop_E04" },
    KnownMap { id: 18, folder_name: "SCCA_Coop_E05" },
    KnownMap { id: 19, folder_name: "SCCA_Coop_E06" },
    // Cybran (SCCA R)
    KnownMap { id: 20, folder_name: "SCCA_Coop_R01" },
    KnownMap { id: 21, folder_name: "SCCA_Coop_R02" },
    KnownMap { id: 22, folder_name: "SCCA_Coop_R03" },
    KnownMap { id: 23, folder_name: "SCCA_Coop_R04" },
    KnownMap { id: 24, folder_name: "SCCA_Coop_R05" },
    KnownMap { id: 25, folder_name: "SCCA_Coop_R06" },
    // FAF originals
    KnownMap { id: 26, folder_name: "FAF_Coop_Operation_Prothyon_16" },
    KnownMap { id: 27, folder_name: "FAF_Coop_Operation_Fort_Clarke_Assault" },
    KnownMap { id: 28, folder_name: "FAF_Coop_Operation_Novax_Station_Assault" },
    KnownMap { id: 29, folder_name: "FAF_Coop_Operation_Tight_Spot" },
    KnownMap { id: 30, folder_name: "FAF_Coop_Operation_Black_Day" },
    KnownMap { id: 31, folder_name: "FAF_Coop_Operation_Overlord" },
    KnownMap { id: 49, folder_name: "FAF_Coop_Operation_Tha_Thuum" },
];

pub async fn deploy_all_maps(ports: &Ports, config: &MapConfig) -> Result<DeployResult, DeployError> {
    ports.git
        .checkout(&config.repo_url, &config.git_ref, &config.workdir)
        .await
        .map_err(|e| DeployError::new(format!("git checkout failed: {e}")))?;

    let mut result = DeployResult::default();

    for map in KNOWN_MAPS {
        match deploy_single_map(ports, config, map).await {
            Ok(true)  => result.updated += 1,
            Ok(false) => result.skipped += 1,
            Err(e) => {
                // Mirror CoopMapDeployer.kt: log and continue, don't abort the whole run.
                eprintln!("warning: skipping map {} ({}): {e}", map.id, map.folder_name);
                result.skipped += 1;
            }
        }
    }

    Ok(result)
}

/// Returns `true` if the map was updated, `false` if unchanged.
async fn deploy_single_map(ports: &Ports, config: &MapConfig, map: &KnownMap) -> Result<bool, DeployError> {
    let map_dir = config.workdir.join(map.folder_name);
    let files = ports.fs.list_files(&map_dir).await
        .map_err(|e| DeployError::new(format!("cannot list {}: {e}", map.folder_name)))?;

    if files.is_empty() {
        return Err(DeployError::new(format!("no files found for {}", map.folder_name)));
    }

    let entries = collect_entries(ports, &files, &map_dir, map.folder_name).await?;
    let new_checksum = compute_entries_checksum(ports, &entries).await?;

    let current = ports.db.get_map(map.id).await
        .map_err(|e| DeployError::new(e.to_string()))?;

    // Compare against stored checksum (stored as the filename hash — simplified approach).
    // If the map doesn't exist yet, always deploy.
    if let Some(ref existing) = current {
        if existing.checksum == new_checksum {
            return Ok(false);
        }
    }

    let next_version = current.as_ref().map(|m| m.version + 1).unwrap_or(1);
    let zip_name = format!("{}.v{:04}.zip", map.folder_name, next_version);
    let zip_path = config.map_dir.join(&zip_name);

    if !config.dry_run {
        ports.fs.write_zip(entries, &zip_path).await
            .map_err(|e| DeployError::new(format!("zip write failed: {e}")))?;
        ports.db.update_map(map.id, next_version, &format!("maps/{zip_name}"), &new_checksum).await
            .map_err(|e| DeployError::new(e.to_string()))?;
    }

    Ok(true)
}

async fn collect_entries(ports: &Ports, files: &[PathBuf], base: &Path, folder_name: &str) -> Result<Vec<ZipEntry>, DeployError> {
    let mut entries = Vec::new();
    for file in files {
        let content = ports.fs.read_file(file).await
            .map_err(|e| DeployError::new(format!("cannot read {}: {e}", file.display())))?;
        let relative = file.strip_prefix(base).unwrap_or(file);
        entries.push(ZipEntry {
            path: PathBuf::from(folder_name).join(relative),
            content,
        });
    }
    Ok(entries)
}

async fn compute_entries_checksum(ports: &Ports, entries: &[ZipEntry]) -> Result<String, DeployError> {
    let combined: Vec<u8> = entries.iter().flat_map(|e| e.content.iter().copied()).collect();
    ports.fs.compute_md5(&combined).await
        .map_err(|e| DeployError::new(format!("md5 failed: {e}")))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::infra::test_support::TestPorts;

    fn test_config() -> MapConfig {
        MapConfig {
            repo_url: "fake".into(),
            git_ref: "master".into(),
            workdir: PathBuf::from("/repo"),
            map_dir: PathBuf::from("/maps"),
            dry_run: false,
        }
    }

    #[tokio::test]
    async fn changed_map_is_updated() {
        let tp = TestPorts::new();
        tp.fs.seed_file("/repo/X1CA_Coop_001/map.lua", b"content v2".as_ref());

        let result = deploy_all_maps(&tp.ports(), &test_config()).await.unwrap();

        assert!(result.updated >= 1);
        let map = tp.db.get_map_sync(1).unwrap();
        assert_eq!(map.version, 1);
        assert!(map.filename.contains("X1CA_Coop_001"));
    }

    #[tokio::test]
    async fn unchanged_map_is_skipped() {
        let tp = TestPorts::new();
        tp.fs.seed_file("/repo/X1CA_Coop_001/map.lua", b"content".as_ref());

        deploy_all_maps(&tp.ports(), &test_config()).await.unwrap();
        let version_after_first = tp.db.get_map_sync(1).unwrap().version;

        deploy_all_maps(&tp.ports(), &test_config()).await.unwrap();
        let version_after_second = tp.db.get_map_sync(1).unwrap().version;

        assert_eq!(version_after_first, version_after_second);
    }

    #[tokio::test]
    async fn dry_run_does_not_write_db() {
        let tp = TestPorts::new();
        tp.fs.seed_file("/repo/X1CA_Coop_001/map.lua", b"content".as_ref());

        let mut config = test_config();
        config.dry_run = true;
        deploy_all_maps(&tp.ports(), &config).await.unwrap();

        assert!(tp.db.get_map_sync(1).is_none());
    }

    #[tokio::test]
    async fn missing_map_folder_is_skipped_not_fatal() {
        let tp = TestPorts::new();
        let result = deploy_all_maps(&tp.ports(), &test_config()).await.unwrap();
        assert_eq!(result.updated, 0);
    }
}
