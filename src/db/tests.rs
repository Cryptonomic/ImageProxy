use crate::{
    moderation::{ModerationCategories, ModerationService},
    utils::sha256,
};
use async_trait::async_trait;
use std::{collections::HashMap, sync::Mutex};
use uuid::Uuid;

use super::{DatabaseProvider, DbModerationRow, DbReportRow, Result};

pub struct DummyDatabase {
    report_store: Mutex<HashMap<String, DbReportRow>>,
    moderation_store: Mutex<HashMap<String, DbModerationRow>>,
}

impl Default for DummyDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl DummyDatabase {
    pub fn new() -> Self {
        DummyDatabase {
            report_store: Mutex::new(HashMap::new()),
            moderation_store: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl DatabaseProvider for DummyDatabase {
    async fn add_report(
        &self,
        id: &Uuid,
        url: &str,
        categories: &[ModerationCategories],
    ) -> Result<()> {
        let url_hash = sha256(url.as_bytes());
        let updated_at = chrono::Utc::now();
        let row = DbReportRow {
            id: id.to_string(),
            url: String::from(url),
            categories: Vec::from(categories),
            updated_at,
        };

        let mut report_store = self.report_store.lock().unwrap();
        report_store.insert(url_hash, row);
        Ok(())
    }

    async fn get_reports(&self) -> Result<Vec<DbReportRow>> {
        let report_store = self.report_store.lock().unwrap();
        let values: Vec<DbReportRow> = report_store
            .iter()
            .map(|(_, value)| value.clone())
            .collect();
        Ok(values)
    }

    async fn update_moderation_result(
        &self,
        url: &str,
        provider: ModerationService,
        blocked: bool,
        categories: &[ModerationCategories],
    ) -> Result<()> {
        self.add_moderation_result(url, provider, blocked, categories)
            .await
    }

    async fn add_moderation_result(
        &self,
        url: &str,
        provider: ModerationService,
        blocked: bool,
        categories: &[ModerationCategories],
    ) -> Result<()> {
        let url_hash = sha256(url.as_bytes());
        let row = DbModerationRow {
            blocked,
            categories: Vec::from(categories),
            provider,
            url: String::from(url),
        };

        let mut moderation_store = self.moderation_store.lock().unwrap();
        moderation_store.insert(url_hash, row);
        let _store_len = moderation_store.len();
        Ok(())
    }

    async fn get_moderation_result(&self, urls: &[String]) -> Result<Vec<DbModerationRow>> {
        let moderation_store = self.moderation_store.lock().unwrap();
        let urls = Vec::from(urls);
        let result: Vec<DbModerationRow> = urls
            .iter()
            .map(|url| {
                let url_hash = sha256(url.as_bytes());
                moderation_store.get(&url_hash)
            })
            .filter(|row| row.is_some())
            .map(|row| row.unwrap().clone())
            .collect();
        Ok(result)
    }
}

#[tokio::test]
async fn test_dummy_database_moderation_fns() {
    let db = DummyDatabase::new();
    let url = "http://localhost/test.png".to_string();
    let result = db.get_moderation_result(&[url.clone()]).await.unwrap();
    assert_eq!(result.len(), 0);

    let _ = db
        .add_moderation_result(
            url.as_str(),
            ModerationService::Unknown,
            true,
            &[ModerationCategories::Alcohol],
        )
        .await;
    let result = db.get_moderation_result(&[url.clone()]).await.unwrap();
    assert_eq!(result.len(), 1);
    let row = result.first().unwrap();
    assert!(row.blocked);
    assert_eq!(row.categories[0], ModerationCategories::Alcohol);
    assert_eq!(row.url, url);
    assert_eq!(row.provider, ModerationService::Unknown);

    let _ = db
        .update_moderation_result(
            url.as_str(),
            ModerationService::Unknown,
            true,
            &[ModerationCategories::Alcohol, ModerationCategories::Drugs],
        )
        .await;

    let result = db.get_moderation_result(&[url.clone()]).await.unwrap();
    assert_eq!(result.len(), 1);
    let row = result.first().unwrap();
    assert!(row.blocked);
    assert_eq!(row.categories.len(), 2);
    assert_eq!(row.categories[0], ModerationCategories::Alcohol);
    assert_eq!(row.categories[1], ModerationCategories::Drugs);
    assert_eq!(row.url, url);
    assert_eq!(row.provider, ModerationService::Unknown);
}

#[tokio::test]
async fn test_dummy_database_report_fns() {
    let db = DummyDatabase::new();
    let url = "http://localhost/test.png".to_string();
    let id = Uuid::new_v4();
    let result = db.get_reports().await.unwrap();
    assert_eq!(result.len(), 0);

    let _ = db
        .add_report(&id, url.as_str(), &[ModerationCategories::Alcohol])
        .await;
    let result = db.get_reports().await.unwrap();
    assert_eq!(result.len(), 1);
    let row = result.first().unwrap();
    assert_eq!(row.url, url);
    assert_eq!(row.categories[0], ModerationCategories::Alcohol);
}
