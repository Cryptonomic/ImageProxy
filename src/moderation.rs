use async_trait::async_trait;
use log::warn;
use serde::{Deserialize, Serialize};

use crate::{aws::Rekognition, config::Configuration, document::Document, rpc::error::Errors};

#[derive(PartialEq)]
pub enum SupportedMimeTypes {
    ImageJpeg,
    ImagePng,
    ImageGif,
    ImageBmp,
    ImageTiff,
    Unsupported,
}

impl SupportedMimeTypes {
    pub fn from_str(s: &str) -> Self {
        match s {
            "image/jpeg" => SupportedMimeTypes::ImageJpeg,
            "image/jpg" => SupportedMimeTypes::ImageJpeg,
            "image/png" => SupportedMimeTypes::ImagePng,
            "image/tiff" => SupportedMimeTypes::ImageTiff,
            "image/gif" => SupportedMimeTypes::ImageGif,
            "image/bmp" => SupportedMimeTypes::ImageBmp,
            "image/x-ms-bmp" => SupportedMimeTypes::ImageBmp,
            _ => SupportedMimeTypes::Unsupported,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModerationCategories {
    ExplicitNudity,
    Suggestive,
    Violence,
    VisuallyDisturbing,
    Rude,
    Drugs,
    Tobacco,
    Alcohol,
    Gambling,
    Hate,
    Unknown,
}
#[derive(Clone)]
pub struct ModerationResponse {
    pub categories: Vec<ModerationCategories>,
    pub provider: ModerationService,
}

/// A trait that all moderation services must implement
#[async_trait]
pub trait ModerationProvider: Send + Sync {
    async fn moderate(self: &Self, document: &Document) -> Result<ModerationResponse, Errors>;
    fn supported_types(self: &Self) -> Vec<SupportedMimeTypes>;
    fn max_document_size(self: &Self) -> u64;
}

#[derive(Clone)]
pub struct NullProvider {}

#[async_trait]
impl ModerationProvider for NullProvider {
    async fn moderate(self: &Self, _: &Document) -> Result<ModerationResponse, Errors> {
        Ok(ModerationResponse {
            categories: Vec::new(),
            provider: ModerationService::None,
        })
    }

    fn supported_types(self: &Self) -> Vec<SupportedMimeTypes> {
        vec![SupportedMimeTypes::ImageJpeg, SupportedMimeTypes::ImagePng]
    }

    fn max_document_size(self: &Self) -> u64 {
        5242880
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum ModerationService {
    Aws,
    Unknown,
    None,
}

impl ModerationService {
    pub fn get_provider(
        config: &Configuration,
    ) -> Result<Box<dyn ModerationProvider + Send + Sync>, Box<dyn std::error::Error + Send + Sync>>
    {
        match config.moderation.provider {
            ModerationService::Aws => match &config.moderation.aws {
                Some(aws_config) => {
                    let s = Rekognition::new(&aws_config.region)?;
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
