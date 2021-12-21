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

impl DummyDatabase {
    #[allow(dead_code)]
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
        Ok(())
    }

    async fn get_moderation_result(&self, url: &[String]) -> Result<Vec<DbModerationRow>> {
        let moderation_store = self.moderation_store.lock().unwrap();
        let url_hash = sha256(url[0].as_bytes());
        let row = moderation_store.get(&url_hash);
        let result: Vec<DbModerationRow> = if row.is_some() {
            vec![row.unwrap().clone()]
        } else {
            vec![]
        };
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_moderation_fns() {
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
        let row = result.get(0).unwrap();
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
        let row = result.get(0).unwrap();
        assert!(row.blocked);
        assert_eq!(row.categories.len(), 2);
        assert_eq!(row.categories[0], ModerationCategories::Alcohol);
        assert_eq!(row.categories[1], ModerationCategories::Drugs);
        assert_eq!(row.url, url);
        assert_eq!(row.provider, ModerationService::Unknown);
    }

    #[tokio::test]
    async fn test_report_fns() {
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
        let row = result.get(0).unwrap();
        assert_eq!(row.url, url);
        assert_eq!(row.categories[0], ModerationCategories::Alcohol);
    }
}
