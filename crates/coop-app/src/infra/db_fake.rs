use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use coop_domain::models::{Campaign, CoopMap, PatchRecord};

use crate::ports::db::{DbPort, DbResult};

/// In-memory database. No SQL, no network. Use in tests and with FAKE_DB=1.
#[derive(Default)]
pub struct FakeDb {
    maps: Mutex<HashMap<i32, CoopMap>>,
    patches: Mutex<HashMap<i32, PatchRecord>>,
    campaigns: Mutex<HashMap<i32, Campaign>>,
}

impl FakeDb {
    pub fn seed_map(&self, map: CoopMap) {
        self.maps.lock().unwrap().insert(map.id, map);
    }

    pub fn seed_patch(&self, record: PatchRecord) {
        self.patches.lock().unwrap().insert(record.file_id, record);
    }

    pub fn seed_campaign(&self, campaign: Campaign) {
        self.campaigns.lock().unwrap().insert(campaign.id, campaign);
    }
}

#[async_trait]
impl DbPort for FakeDb {
    async fn get_map(&self, map_id: i32) -> DbResult<Option<CoopMap>> {
        Ok(self.maps.lock().unwrap().get(&map_id).cloned())
    }

    async fn update_map(&self, map_id: i32, version: i32, filename: &str, checksum: &str) -> DbResult<()> {
        let mut maps = self.maps.lock().unwrap();
        let map = maps.entry(map_id).or_insert_with(|| CoopMap {
            id: map_id,
            name: format!("map_{map_id}"),
            version: 0,
            filename: String::new(),
            checksum: String::new(),
        });
        map.version = version;
        map.filename = filename.to_string();
        map.checksum = checksum.to_string();
        Ok(())
    }

    async fn get_patch_record(&self, file_id: i32) -> DbResult<Option<PatchRecord>> {
        Ok(self.patches.lock().unwrap().get(&file_id).cloned())
    }

    async fn upsert_patch(&self, record: PatchRecord) -> DbResult<()> {
        self.patches.lock().unwrap().insert(record.file_id, record);
        Ok(())
    }

    async fn list_campaigns(&self) -> DbResult<Vec<Campaign>> {
        Ok(self.campaigns.lock().unwrap().values().cloned().collect())
    }

    async fn upsert_campaign(&self, campaign: Campaign) -> DbResult<()> {
        self.campaigns.lock().unwrap().insert(campaign.id, campaign);
        Ok(())
    }
}

impl FakeDb {
    pub fn get_map_sync(&self, map_id: i32) -> Option<CoopMap> {
        self.maps.lock().unwrap().get(&map_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn update_map_creates_if_missing() {
        let db = FakeDb::default();
        db.update_map(42, 3, "maps/foo.v0003.zip", "abc123").await.unwrap();
        let map = db.get_map(42).await.unwrap().unwrap();
        assert_eq!(map.version, 3);
        assert_eq!(map.filename, "maps/foo.v0003.zip");
    }

    #[tokio::test]
    async fn seed_and_retrieve_patch() {
        let db = FakeDb::default();
        db.seed_patch(PatchRecord { file_id: 1, name: "voice.nx2".into(), md5: "abc".into(), version: 2 });
        let record = db.get_patch_record(1).await.unwrap().unwrap();
        assert_eq!(record.md5, "abc");
    }
}
