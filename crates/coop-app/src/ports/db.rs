use async_trait::async_trait;
use coop_domain::{errors::DbError, models::{Campaign, CoopMap, PatchRecord}};

pub type DbResult<T> = Result<T, DbError>;

#[async_trait]
pub trait DbPort: Send + Sync {
    async fn get_map(&self, map_id: i32) -> DbResult<Option<CoopMap>>;
    async fn update_map(&self, map_id: i32, version: i32, filename: &str, checksum: &str) -> DbResult<()>;

    async fn get_patch_record(&self, file_id: i32) -> DbResult<Option<PatchRecord>>;
    async fn upsert_patch(&self, record: PatchRecord) -> DbResult<()>;

    async fn list_campaigns(&self) -> DbResult<Vec<Campaign>>;
    async fn upsert_campaign(&self, campaign: Campaign) -> DbResult<()>;
}
