use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoopMap {
    pub id: i32,
    pub name: String,
    pub version: i32,
    pub filename: String,
    pub checksum: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caller_identity_has_role_true() {
        let identity = CallerIdentity {
            user_id: 1,
            username: "alice".into(),
            roles: vec!["COOP_DEPLOYER".into(), "USER".into()],
        };
        assert!(identity.has_role("COOP_DEPLOYER"));
    }

    #[test]
    fn caller_identity_has_role_false() {
        let identity = CallerIdentity {
            user_id: 1,
            username: "alice".into(),
            roles: vec!["USER".into()],
        };
        assert!(!identity.has_role("COOP_DEPLOYER"));
    }

    #[test]
    fn deploy_result_default_is_zero() {
        let result = DeployResult::default();
        assert_eq!(result.updated, 0);
        assert_eq!(result.skipped, 0);
    }

    #[test]
    fn coop_map_fields() {
        let map = CoopMap { id: 42, name: "Fort Clarke".into(), version: 3, filename: "maps/fort.v0003.zip".into(), checksum: "abc".into() };
        assert_eq!(map.id, 42);
        assert_eq!(map.version, 3);
    }

    #[test]
    fn campaign_map_ids_ordered() {
        let campaign = Campaign { id: 1, name: "UEF".into(), map_ids: vec![10, 20, 30] };
        assert_eq!(campaign.map_ids, vec![10, 20, 30]);
    }
}
