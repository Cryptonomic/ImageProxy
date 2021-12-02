use crate::{
    config::DatabaseConfig,
    moderation::{ModerationCategories, ModerationService},
    utils::sha256,
};
use async_trait::async_trait;
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use chrono::{DateTime, Utc};
use log::debug;
use std::time::Duration;
use tokio_postgres::NoTls;
use uuid::Uuid;

use super::{DatabaseProvider, DbModerationRow, DbReportRow, Result};

#[derive(Clone)]
pub struct PostgresDatabase {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

impl PostgresDatabase {
    pub async fn new(config: &DatabaseConfig) -> Result<PostgresDatabase> {
        let connection_string = format!(
            "postgresql://{}:{}@{}:{}",
            config.username, config.password, config.host, config.port
        );
        let pg_mgr = PostgresConnectionManager::new_from_stringlike(
            connection_string,
            tokio_postgres::NoTls,
        )
        .unwrap();

        Ok(PostgresDatabase {
            pool: Pool::builder()
                .connection_timeout(Duration::new(config.pool_connection_timeout, 0))
                .min_idle(Some(config.pool_idle_connections))
                .max_size(config.pool_max_connections)
                .build(pg_mgr)
                .await
                .unwrap(),
        })
    }
}

#[async_trait]
impl DatabaseProvider for PostgresDatabase {
    async fn add_report(
        &self,
        id: &Uuid,
        url: &str,
        categories: &[ModerationCategories],
    ) -> Result<()> {
        let id = id.to_string();
        let url_hash = sha256(url.as_bytes());
        let timestamp = chrono::Utc::now();
        let cat_str =
            serde_json::to_string(categories).unwrap_or_else(|_| String::from("json_error"));
        let conn = self.pool.get().await?;
        conn.execute(
            "INSERT INTO report (id, url, categories, url_hash, updated_at)
        VALUES($1, $2, $3, $4, $5) 
        ON CONFLICT (id) 
        DO NOTHING;",
            &[&id, &url, &cat_str, &url_hash, &timestamp],
        )
        .await?;
        Ok(())
    }

    async fn get_reports(&self) -> Result<Vec<DbReportRow>> {
        let conn = self.pool.get().await?;
        let results = conn
            .query("SELECT id, url, categories, updated_at from report;", &[])
            .await?;

        Ok(results
            .iter()
            .map(|r| {
                let categories: &str = r.get("categories");
                let id: &str = r.get("id");
                let url: &str = r.get("url");
                let categories = serde_json::from_str::<Vec<ModerationCategories>>(categories)
                    .unwrap_or_default();
                let updated_at: DateTime<Utc> = r.get("updated_at");
                DbReportRow {
                    id: String::from(id),
                    url: String::from(url),
                    categories,
                    updated_at,
                }
            })
            .collect())
    }

    async fn update_moderation_result(
        &self,
        url: &str,
        provider: ModerationService,
        blocked: bool,
        categories: &[ModerationCategories],
    ) -> Result<()> {
        let url_hash = sha256(url.as_bytes());
        let timestamp = chrono::Utc::now();
        let cat_str =
            serde_json::to_string(categories).unwrap_or_else(|_| String::from("json_error"));
        let provider_str =
            serde_json::to_string(&provider).unwrap_or_else(|_| String::from("json_error"));
        let conn = self.pool.get().await?;
        conn.execute(
            "UPDATE documents
            SET blocked    = $1,
                provider   = $2,
                categories = $3,
                updated_at = $4
            WHERE url_hash = $5;",
            &[&blocked, &provider_str, &cat_str, &timestamp, &url_hash],
        )
        .await?;
        Ok(())
    }

    async fn add_moderation_result(
        &self,
        url: &str,
        provider: ModerationService,
        blocked: bool,
        categories: &[ModerationCategories],
    ) -> Result<()> {
        let url_hash = sha256(url.as_bytes());
        let doc_hash = ""; //FIXME
        let timestamp = chrono::Utc::now();
        let provider_str =
            serde_json::to_string(&provider).unwrap_or_else(|_| String::from("json_error"));
        let cat_str =
            serde_json::to_string(categories).unwrap_or_else(|_| String::from("json_error"));
        let conn = self.pool.get().await?;
        conn.execute("INSERT INTO documents (url_hash, url, blocked, provider, categories, doc_hash, updated_at)
        VALUES($1, $2, $3, $4, $5, $6, $7) 
        ON CONFLICT (url_hash) 
        DO NOTHING;", &[&url_hash, &url, &blocked, &provider_str, &cat_str, &doc_hash, &timestamp]).await?;
        Ok(())
    }

    async fn get_moderation_result(&self, url: &[String]) -> Result<Vec<DbModerationRow>> {
        let url_hashes: Vec<String> = url.iter().map(|u| sha256(u.as_bytes())).collect();
        let conn = self.pool.get().await?;
        let results = conn
            .query(
                "SELECT blocked, categories, provider, url from documents 
            WHERE documents.url_hash = ANY($1);",
                &[&url_hashes],
            )
            .await?;

        debug!("Retrieved {} rows.", results.len());
        if results.is_empty() {
            return Ok(Vec::new());
        }

        Ok(results
            .iter()
            .map(|r| {
                let blocked: bool = r.get("blocked");
                let categories: &str = r.get("categories");
                let provider: &str = r.get("provider");
                let url: &str = r.get("url");

                let categories = serde_json::from_str::<Vec<ModerationCategories>>(categories)
                    .unwrap_or_default();
                let provider = serde_json::from_str::<ModerationService>(provider)
                    .unwrap_or(ModerationService::Unknown);
                DbModerationRow {
                    blocked,
                    categories,
                    provider,
                    url: String::from(url),
                }
            })
            .collect())
    }
}
