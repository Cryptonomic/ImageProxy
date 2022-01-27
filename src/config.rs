use std::net::Ipv4Addr;

use hocon::{Error, HoconLoader};
use serde::Deserialize;

use crate::{cache::CacheConfig, moderation::ModerationService};

#[derive(Deserialize, Clone)]
#[allow(dead_code)]
pub struct Cors {
    pub origin: String,
}

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
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub db: String,
    pub pool_max_connections: u32,
    pub pool_idle_connections: u32,
    pub pool_connection_timeout: u64,
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

#[derive(Deserialize, Clone, Debug)]
pub struct ApiKey {
    pub name: String,
    pub key: String,
}

#[derive(Deserialize, Clone)]
pub struct SecurityConfig {
    pub api_keys: Vec<ApiKey>,
}

#[derive(Deserialize, Clone)]
#[allow(dead_code)]
pub struct Configuration {
    pub ipfs: Host,
    pub cors: Cors,
    pub workers: u16,
    pub bind_address: Ipv4Addr,
    pub port: u16,
    pub timeout: u64,
    pub metrics_enabled: bool,
    pub dashboard_enabled: bool,
    pub max_document_size: Option<u64>,
    pub client_useragent: Option<String>,
    pub security: SecurityConfig,
    pub database: DatabaseConfig,
    pub moderation: ModerationConfig,
    pub cache_config: CacheConfig,
}

impl Configuration {
    pub fn load() -> Result<Configuration, Error> {
        let conf: Configuration = HoconLoader::new().load_file("proxy.conf")?.resolve()?;
        Ok(conf)
    }
}
