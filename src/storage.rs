use async_trait::async_trait;
use log::warn;
use serde::{Deserialize, Serialize};

use crate::{
    aws::s3::S3, config::Configuration, document::Document, moderation::ModerationService,
    rpc::error::Errors,
};

/// A trait that all storage services must implement
#[async_trait]
pub trait StorageProvider: Send + Sync {
    async fn store(&self, document: &Document) -> Result<(), Errors>;
}

#[derive(Clone)]
pub struct NullProvider {}

#[async_trait]
impl StorageProvider for NullProvider {
    async fn store(&self, _: &Document) -> Result<(), Errors> {
        Ok(())
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum StorageService {
    Aws,
    Unknown,
    None,
}

impl StorageService {
    pub async fn get_provider(
        config: &Configuration,
    ) -> Result<Box<dyn StorageProvider + Send + Sync>, Box<dyn std::error::Error + Send + Sync>>
    {
        match config.moderation.provider {
            ModerationService::Aws => match &config.moderation.aws {
                Some(aws_config) => {
                    let s3_permits = &aws_config
                        .video
                        .as_ref()
                        .expect("video config not set for aws")
                        .s3_jobs;
                    let s = S3::new(&aws_config.region, "nft-mediaproxy", s3_permits).await;
                    Ok(Box::new(s))
                }
                None => Err("Moderation provider configuration is missing".into()),
            },
            _ => {
                // Used when no valid moderation provider is available
                warn!("No valid moderation provider is available. Using a block always provider");
                Ok(Box::new(NullProvider {}))
            }
        }
    }
}
