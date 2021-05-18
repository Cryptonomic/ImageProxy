use hocon::{Error, HoconLoader};
use serde::Deserialize;

use crate::moderation::ModerationService;

#[derive(Deserialize, Clone)]
#[allow(dead_code)]
pub struct Host {
    pub protocol: String,
    pub host: String,
    pub port: u16,
    pub path: String,
}
#[derive(Deserialize, Clone)]
#[allow(dead_code)]
pub struct Database {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub db: String,
}
#[derive(Deserialize, Clone)]
pub struct AwsConfig {
    pub region: String,
}

#[derive(Deserialize, Clone)]
pub struct ModerationConfig {
    pub provider: ModerationService,
    pub aws: Option<AwsConfig>,
    pub labels: Vec<String>, //TODO
}

#[derive(Deserialize, Clone)]
#[allow(dead_code)]
pub struct Configuration {
    pub ipfs: Host,
    pub workers: u16,
    pub port: u16,
    pub metrics_enabled: bool,
    pub max_document_size: Option<u64>,
    pub api_keys: Vec<String>,
    pub database: Database,
    pub moderation: ModerationConfig,
}

impl Configuration {
    pub fn load() -> Result<Configuration, Error> {
        let conf: Configuration = HoconLoader::new().load_file("proxy.conf")?.resolve()?;
        Ok(conf)
    }
}
