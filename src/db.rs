extern crate crypto;
use std::{net::IpAddr, net::Ipv4Addr, time::Duration, usize};

use crate::{
    moderation::{ModerationCategories, ModerationService},
    Configuration,
};
use bb8::Pool;
use bb8_postgres::PostgresConnectionManager;
use chrono::{DateTime, Utc};
use futures::future::join_all;
use std::collections::HashSet;

use log::debug;
use tokio_postgres::NoTls;

use crypto::{digest::Digest, sha2::Sha512};
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
    pub apikey: String,
    pub ip_addr: IpAddr,
}

pub struct ReportData {
    pub url: String,
    pub url_hash: String,
    pub categories: Vec<ModerationCategories>,
    pub num_reports: usize,
}

impl Database {
    fn sha512(input: &[u8]) -> String {
        let mut hasher = Sha512::new();
        hasher.input(input);
        hasher.result_str()
    }

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
                .connection_timeout(Duration::new(10, 0))
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
        addr: &IpAddr,
        apikey: &String,
    ) -> Result<()> {
        let id = id.to_string();
        let url_hash = Database::sha512(url.as_bytes());
        let timestamp = chrono::Utc::now();
        let cat_str = serde_json::to_string(categories).unwrap_or(String::from("json_error"));
        let ip_addr = addr.to_string();
        let conn = self.pool.get().await?;
        conn.execute(
            "INSERT INTO report (id, url, categories, url_hash, updated_at, ip_addr, apikey)
        VALUES($1, $2, $3, $4, $5, $6, $7) 
        ON CONFLICT (id) 
        DO NOTHING;",
            &[&id, &url, &cat_str, &url_hash, &timestamp, &ip_addr, apikey],
        )
        .await?;
        Ok(())
    }

    pub async fn get_reports(&self) -> Result<Vec<ReportRow>> {
        let conn = self.pool.get().await?;
        let results = conn
            .query(
                "SELECT id, url, categories, updated_at, apikey, ip_addr from report;",
                &[],
            )
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
                let apikey: &str = r.get("apikey");
                let string_addr: &str = r.get("ip_addr");
                let ip_addr: IpAddr = string_addr
                    .parse()
                    .unwrap_or(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
                ReportRow {
                    id: String::from(id),
                    url: String::from(url),
                    apikey: String::from(apikey),
                    categories,
                    updated_at,
                    ip_addr,
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
        let url_hash = Database::sha512(url.as_bytes());
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
        let url_hash = Database::sha512(url.as_bytes());
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

    pub async fn get_user_moderation_result(
        &self,
        urls: &Vec<String>,
        max_report_strikes: usize,
    ) -> Result<Vec<DocumentCacheRow>> {
        let report_results: Vec<Result<ReportData>> =
            join_all(urls.iter().map(|url| self.get_user_moderation_for_url(url))).await;
        Ok(report_results
            .iter()
            .filter(|r| if let Ok(_) = r { true } else { false })
            .map(|r| {
                let data = r.as_ref().unwrap();
                DocumentCacheRow {
                    url: data.url.clone(),
                    blocked: data.num_reports > max_report_strikes,
                    categories: data.categories.clone(),
                    provider: ModerationService::Reports,
                }
            })
            .collect())
    }

    pub async fn get_moderation_result(&self, url: &Vec<String>) -> Result<Vec<DocumentCacheRow>> {
        let url_hashes: Vec<String> = url.iter().map(|u| Database::sha512(u.as_bytes())).collect();
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
                let categories = serde_json::from_str::<HashSet<ModerationCategories>>(&categories)
                    .unwrap_or(HashSet::new());
                let provider = serde_json::from_str::<ModerationService>(&provider)
                    .unwrap_or(ModerationService::Unknown);
                DocumentCacheRow {
                    blocked,
                    categories: categories.clone().into_iter().collect(),
                    provider: provider,
                    url: String::from(url),
                }
            })
            .collect())
    }

    async fn get_user_moderation_for_url(&self, url: &String) -> Result<ReportData> {
        let url_hash = Database::sha512(url.as_bytes());
        let num_reports = self.get_num_reports_for_hash(&url_hash).await?;
        let categories = self
            .get_reported_categories_for_hash(&url_hash)
            .await?
            .into_iter()
            .collect();
        Ok(ReportData {
            url: url.clone(),
            url_hash,
            num_reports,
            categories,
        })
    }

    async fn get_num_reports_for_hash(&self, url_hash: &String) -> Result<usize> {
        Ok(self
            .pool
            .get()
            .await?
            .query(
                "select apikey, ip_addr from report where url_hash=$1 GROUP BY apikey, ip_addr;",
                &[url_hash],
            )
            .await?
            .len())
    }

    async fn get_reported_categories_for_hash(
        &self,
        url_hash: &String,
    ) -> Result<HashSet<ModerationCategories>> {
        let res = self
            .pool
            .get()
            .await?
            .query(
                "select categories from report where url_hash=$1;",
                &[url_hash],
            )
            .await?;
        let res = res
            .iter()
            .map(|r| {
                let categories: &str = r.get("categories");
                serde_json::from_str::<HashSet<ModerationCategories>>(&categories)
                    .unwrap_or(HashSet::new())
            })
            .fold(HashSet::new(), |mut sum, val| {
                sum.extend(val);
                sum
            })
            .into_iter()
            .collect();

        Ok(res)
    }
}
