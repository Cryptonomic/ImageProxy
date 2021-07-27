extern crate crypto;
use std::time::Duration;

use crate::{Configuration, moderation::{ModerationCategories, ModerationService}, utils::sha512};
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use chrono::{DateTime, Utc};
use log::debug;
use tokio_postgres::NoTls;

use uuid::Uuid;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

#[derive(Clone)]
pub struct Database {
    pool: Pool<PostgresConnectionManager<NoTls>>,
}

pub struct DocumentCacheRow {
    pub blocked: bool,
    pub categories: Vec<ModerationCategories>,
    pub provider: ModerationService,
    pub url: String,
}

pub struct ReportRow {
    pub id: String,
    pub url: String,
    pub categories: Vec<ModerationCategories>,
    pub updated_at: DateTime<Utc>,
}

impl Database {
    pub async fn new(conf: &Configuration) -> Result<Database> {
        let connection_string = format!(
            "postgresql://{}:{}@{}:{}",
            conf.database.username, conf.database.password, conf.database.host, conf.database.port
        );
        let pg_mgr = PostgresConnectionManager::new_from_stringlike(
            connection_string,
            tokio_postgres::NoTls,
        )
        .unwrap();

        Ok(Database {
            pool: Pool::builder()
                .connection_timeout(Duration::new(30, 0))
                .min_idle(Some(2))
                .max_size(16)
                .build(pg_mgr)
                .await
                .unwrap(),
        })
    }

    pub async fn add_report(
        &self,
        id: &Uuid,
        url: &str,
        categories: &Vec<ModerationCategories>,
    ) -> Result<()> {
        let id = id.to_string();
        let url_hash = sha512(url.as_bytes());
        let timestamp = chrono::Utc::now();
        let cat_str = serde_json::to_string(categories).unwrap_or(String::from("json_error"));
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

    pub async fn get_reports(&self) -> Result<Vec<ReportRow>> {
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
                let categories = serde_json::from_str::<Vec<ModerationCategories>>(&categories)
                    .unwrap_or(Vec::new());
                let updated_at: DateTime<Utc> = r.get("updated_at");
                ReportRow {
                    id: String::from(id),
                    url: String::from(url),
                    categories: categories,
                    updated_at: updated_at,
                }
            })
            .collect())
    }

    pub async fn update_moderation_result(
        &self,
        url: &str,
        provider: ModerationService,
        blocked: bool,
        categories: &Vec<ModerationCategories>,
    ) -> Result<()> {
        let url_hash = sha512(url.as_bytes());
        let timestamp = chrono::Utc::now();
        let cat_str = serde_json::to_string(categories).unwrap_or(String::from("json_error"));
        let provider_str = serde_json::to_string(&provider).unwrap_or(String::from("json_error"));
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

    pub async fn add_moderation_result(
        &self,
        url: &str,
        provider: ModerationService,
        blocked: bool,
        categories: &Vec<ModerationCategories>,
    ) -> Result<()> {
        let url_hash = sha512(url.as_bytes());
        let doc_hash = ""; //FIXME
        let timestamp = chrono::Utc::now();
        let provider_str = serde_json::to_string(&provider).unwrap_or(String::from("json_error"));
        let cat_str = serde_json::to_string(categories).unwrap_or(String::from("json_error"));
        let conn = self.pool.get().await?;
        conn.execute("INSERT INTO documents (url_hash, url, blocked, provider, categories, doc_hash, updated_at)
        VALUES($1, $2, $3, $4, $5, $6, $7) 
        ON CONFLICT (url_hash) 
        DO NOTHING;", &[&url_hash, &url, &blocked, &provider_str, &cat_str, &doc_hash, &timestamp]).await?;
        Ok(())
    }

    pub async fn get_moderation_result(&self, url: &Vec<String>) -> Result<Vec<DocumentCacheRow>> {
        let url_hashes: Vec<String> = url.iter().map(|u| sha512(u.as_bytes())).collect();
        let conn = self.pool.get().await?;
        let results = conn
            .query(
                "SELECT blocked, categories, provider, url from documents 
            WHERE documents.url_hash = ANY($1);",
                &[&url_hashes],
            )
            .await?;

        debug!("Retrieved {} rows.", results.len());
        if results.len() == 0 {
            return Ok(Vec::new());
        }

        Ok(results
            .iter()
            .map(|r| {
                let blocked: bool = r.get("blocked");
                let categories: &str = r.get("categories");
                let provider: &str = r.get("provider");
                let url: &str = r.get("url");

                let categories = serde_json::from_str::<Vec<ModerationCategories>>(&categories)
                    .unwrap_or(Vec::new());
                let provider = serde_json::from_str::<ModerationService>(&provider)
                    .unwrap_or(ModerationService::Unknown);
                DocumentCacheRow {
                    blocked,
                    categories: categories.clone(),
                    provider: provider,
                    url: String::from(url),
                }
            })
            .collect())
    }

    pub async fn get_all_moderation_result(&self) -> Result<Vec<DocumentCacheRow>> {
        let conn = self.pool.get().await?;
        let results = conn
            .query(
                "SELECT blocked, categories, provider, url from documents;",
                &[],
            )
            .await?;

        debug!("Retrieved {} rows.", results.len());
        if results.len() == 0 {
            return Ok(Vec::new());
        }

        Ok(results
            .iter()
            .map(|r| {
                let blocked: bool = r.get("blocked");
                let categories: &str = r.get("categories");
                let provider: &str = r.get("provider");
                let url: &str = r.get("url");

                let categories = serde_json::from_str::<Vec<ModerationCategories>>(&categories)
                    .unwrap_or(Vec::new());
                let provider = serde_json::from_str::<ModerationService>(&provider)
                    .unwrap_or(ModerationService::Unknown);
                DocumentCacheRow {
                    blocked,
                    categories: categories.clone(),
                    provider: provider,
                    url: String::from(url),
                }
            })
            .collect())
    }
}
