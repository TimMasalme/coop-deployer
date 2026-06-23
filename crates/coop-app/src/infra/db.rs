use async_trait::async_trait;
use sqlx::MySqlPool;
use sqlx::Row;

use coop_domain::{
    errors::DbError,
    models::{Campaign, CoopMap, PatchRecord},
};

use crate::ports::db::{DbPort, DbResult};

pub struct SqlxDb {
    pool: MySqlPool,
}

impl SqlxDb {
    pub async fn connect(database_url: &str) -> Result<Self, DbError> {
        let pool = MySqlPool::connect(database_url)
            .await
            .map_err(|e| DbError::new(format!("DB connect failed: {e}")))?;
        Ok(Self { pool })
    }
}

#[async_trait]
impl DbPort for SqlxDb {
    async fn get_map(&self, map_id: i32) -> DbResult<Option<CoopMap>> {
        let row = sqlx::query("SELECT id, version, filename FROM coop_map WHERE id = ?")
            .bind(map_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DbError::new(e.to_string()))?;

        Ok(row.map(|r| {
            let id = r.get::<u32, _>("id") as i32;
            // DECIMAL(4,0) in MySQL — read as Decimal, cast to i32
            let version = r.get::<rust_decimal::Decimal, _>("version")
                .try_into().unwrap_or(0i32);
            let filename: String = r.get("filename");
            // Parse short checksum from filename: name.v0001.abc12345.zip
            let checksum = filename
                .trim_end_matches(".zip")
                .rsplit('.')
                .next()
                .unwrap_or("")
                .to_string();
            CoopMap { id, name: String::new(), version, filename, checksum }
        }))
    }

    async fn update_map(&self, map_id: i32, version: i32, filename: &str, _checksum: &str) -> DbResult<()> {
        sqlx::query("UPDATE coop_map SET version = ?, filename = ? WHERE id = ?")
            .bind(version)
            .bind(filename)
            .bind(map_id as u32)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::new(e.to_string()))?;
        Ok(())
    }

    async fn get_patch_record(&self, file_id: i32) -> DbResult<Option<PatchRecord>> {
        let row = sqlx::query(
            r#"SELECT uf.fileId as file_id, uf.name, uf.md5, t.v as version
               FROM (
                   SELECT fileId, MAX(version) AS v
                   FROM updates_coop_files
                   GROUP BY fileId
               ) t
               JOIN updates_coop_files uf ON uf.fileId = t.fileId AND uf.version = t.v
               WHERE uf.fileId = ?"#,
        )
        .bind(file_id as u16)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DbError::new(e.to_string()))?;

        Ok(row.map(|r| PatchRecord {
            file_id: r.get::<u16, _>("file_id") as i32,
            name: r.get("name"),
            md5: r.get("md5"),
            version: r.get("version"),
        }))
    }

    async fn upsert_patch(&self, record: PatchRecord) -> DbResult<()> {
        sqlx::query("DELETE FROM updates_coop_files WHERE fileId = ? AND version = ?")
            .bind(record.file_id as u16)
            .bind(record.version - 1)
            .execute(&self.pool)
            .await
            .map_err(|e| DbError::new(e.to_string()))?;

        sqlx::query(
            "INSERT INTO updates_coop_files (fileId, version, name, md5, obselete) VALUES (?, ?, ?, ?, 0)",
        )
        .bind(record.file_id as u16)
        .bind(record.version)
        .bind(&record.name)
        .bind(&record.md5)
        .execute(&self.pool)
        .await
        .map_err(|e| DbError::new(e.to_string()))?;

        Ok(())
    }

    async fn list_campaigns(&self) -> DbResult<Vec<Campaign>> {
        // Campaigns require a new table — migration needed, to be done with Brutus/Sheikah.
        Ok(vec![])
    }

    async fn upsert_campaign(&self, _campaign: Campaign) -> DbResult<()> {
        Err(DbError::new("campaign table not yet migrated — contact FAF team"))
    }
}
