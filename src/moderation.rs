use async_trait::async_trait;
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
    pub fn from_string(s: &str) -> Self {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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
    async fn moderate(&self, document: &Document) -> Result<ModerationResponse, Errors>;
    fn supported_types(&self) -> Vec<SupportedMimeTypes>;
    fn max_document_size(&self) -> u64;
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub enum ModerationService {
    Aws,
    None,    // Used for empty api results only
    Unknown, // Used when db was a provider value which does not appear here
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
            _ => Err("Unknown moderation provider, check configuration".into()),
        }
    }
}

#[cfg(test)]
pub mod tests {

    use std::{collections::HashMap, sync::Mutex};

    use hyper::body::Bytes;
    use uuid::Uuid;

    use super::*;

    pub struct DummyModerationProvider {
        store: Mutex<HashMap<String, Vec<ModerationCategories>>>,
    }

    impl Default for DummyModerationProvider {
        fn default() -> Self {
            Self::new()
        }
    }

    impl DummyModerationProvider {
        pub fn new() -> Self {
            DummyModerationProvider {
                store: Mutex::new(HashMap::new()),
            }
        }

        pub fn set(&mut self, url: &str, categories: Vec<ModerationCategories>) {
            let mut store = self.store.lock().unwrap();
            store.insert(url.to_string(), categories);
        }
    }

    #[async_trait]
    impl ModerationProvider for DummyModerationProvider {
        async fn moderate(&self, document: &Document) -> Result<ModerationResponse, Errors> {
            let url = &document.url;
            let store = self.store.lock().unwrap();
            let default = &Vec::<ModerationCategories>::new();
            let categories = store.get(url).unwrap_or(default);
            Ok(ModerationResponse {
                categories: categories.clone(),
                provider: ModerationService::None,
            })
        }

        fn supported_types(&self) -> Vec<SupportedMimeTypes> {
            vec![SupportedMimeTypes::ImageJpeg]
        }

        fn max_document_size(&self) -> u64 {
            12
        }
    }

    #[tokio::test]
    async fn test_dummy_moderation_provider() {
        let mut provider = DummyModerationProvider::new();
        let categories = vec![ModerationCategories::Gambling, ModerationCategories::Drugs];
        let document1 = Document {
            id: Uuid::new_v4(),
            content_type: "image/jpg".to_string(),
            content_length: 100_u64,
            bytes: Bytes::new(),
            url: "http://localhost".to_string(),
        };

        assert_eq!(provider.max_document_size(), 12);
        assert_eq!(provider.supported_types().len(), 1);
        assert!(provider
            .supported_types()
            .contains(&SupportedMimeTypes::ImageJpeg));

        // Expect empty response
        let response = provider.moderate(&document1).await;
        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.provider, ModerationService::None);
        assert_eq!(response.categories.len(), 0);

        // Set some result for the document
        provider.set(&document1.url, categories);

        // Expect a non empty response
        let response = provider.moderate(&document1).await;
        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.provider, ModerationService::None);
        assert_eq!(response.categories.len(), 2);
        assert!(response
            .categories
            .contains(&ModerationCategories::Gambling));
        assert!(response.categories.contains(&ModerationCategories::Drugs));
    }
}
