use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoopMap {
    pub id: i32,
    pub name: String,
    pub version: i32,
    pub filename: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Campaign {
    pub id: i32,
    pub name: String,
    pub map_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchRecord {
    pub file_id: i32,
    pub name: String,
    pub md5: String,
    pub version: i32,
}

#[derive(Debug, Clone)]
pub struct ZipEntry {
    pub path: PathBuf,
    pub content: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct Asset {
    pub name: String,
    pub download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeployResult {
    pub updated: u32,
    pub skipped: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallerIdentity {
    pub user_id: i32,
    pub username: String,
    pub roles: Vec<String>,
}

impl CallerIdentity {
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}
