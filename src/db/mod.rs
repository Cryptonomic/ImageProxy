use self::postgres::PostgresDatabase;
use crate::{
    config::DatabaseConfig,
    moderation::{ModerationCategories, ModerationService},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

mod postgres;

type GenericError = Box<dyn std::error::Error + Send + Sync>;
type Result<T> = std::result::Result<T, GenericError>;

pub struct DbModerationRow {
    pub blocked: bool,
    pub categories: Vec<ModerationCategories>,
    pub provider: ModerationService,
    pub url: String,
}

pub struct DbReportRow {
    pub id: String,
    pub url: String,
    pub categories: Vec<ModerationCategories>,
    pub updated_at: DateTime<Utc>,
}

#[async_trait]
pub trait DatabaseProvider {
    async fn add_report(
        &self,
        id: &Uuid,
        url: &str,
        categories: &[ModerationCategories],
    ) -> Result<()>;

    async fn get_reports(&self) -> Result<Vec<DbReportRow>>;

    async fn update_moderation_result(
        &self,
        url: &str,
        provider: ModerationService,
        blocked: bool,
        categories: &[ModerationCategories],
    ) -> Result<()>;

    async fn add_moderation_result(
        &self,
        url: &str,
        provider: ModerationService,
        blocked: bool,
        categories: &[ModerationCategories],
    ) -> Result<()>;

    async fn get_moderation_result(&self, url: &[String]) -> Result<Vec<DbModerationRow>>;
}

pub struct DatabaseFactory;

impl DatabaseFactory {
    pub async fn get_provider(
        config: &DatabaseConfig,
    ) -> Result<Box<dyn DatabaseProvider + Send + Sync>> {
        let db = PostgresDatabase::new(config).await?;
        Ok(Box::new(db))
    }
}
