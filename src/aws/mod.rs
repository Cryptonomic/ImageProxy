use async_trait::async_trait;
use log::{debug, error};
use std::env;

use crate::{
    document::Document,
    moderation::{
        ModerationCategories, ModerationProvider, ModerationResponse, ModerationService,
        SupportedMimeTypes,
    },
    rpc::error::Errors,
};

use aws_sdk_rekognition::error::DetectModerationLabelsError;
use aws_sdk_rekognition::model::Image;
use aws_sdk_rekognition::output::DetectModerationLabelsOutput;
use aws_sdk_rekognition::SdkError;
use aws_sdk_rekognition::{Blob, Client as ClientRekognition, Region};

pub struct Rekognition {
    pub region: String,
}

#[async_trait]
impl ModerationProvider for Rekognition {
    async fn moderate(
        &self,
        document: &Document,
    ) -> Result<crate::moderation::ModerationResponse, Errors> {
        debug!("New Rekognition request");
        match self.get_moderation_labels(&document.bytes).await {
            Ok(result) => {
                let labels = result.moderation_labels.unwrap_or_default();
                let labels = labels
                    .into_iter()
                    .filter(|l| l.name.is_some()) // Remove empty labels
                    .map(|l| {
                        let name = l.name.unwrap(); // This is safe
                        Rekognition::normalize_category(name.as_str())
                    })
                    .filter(|l| *l != ModerationCategories::Unknown)
                    .collect();

                debug!(
                    "Moderation labels for id={}, labels={:?}",
                    document.id, labels
                );
                Ok(ModerationResponse {
                    categories: labels,
                    provider: ModerationService::Aws,
                })
            }
            Err(e) => {
                error!("Moderation failed, reason:{}", e);
                Err(Errors::ModerationFailed)
            }
        }
    }

    fn supported_types(&self) -> Vec<SupportedMimeTypes> {
        vec![SupportedMimeTypes::ImageJpeg, SupportedMimeTypes::ImagePng]
    }

    fn max_document_size(&self) -> u64 {
        5242880 // As per AWS documentation, 5 MB binary limit
    }
}

impl Rekognition {
    pub async fn get_moderation_labels(
        &self,
        bytes: &hyper::body::Bytes,
    ) -> Result<DetectModerationLabelsOutput, SdkError<DetectModerationLabelsError>> {
        let region = Region::new(self.region.clone());
        let shared_config = aws_config::from_env().region(region).load().await;
        let client = ClientRekognition::new(&shared_config);
        let req = client.detect_moderation_labels();
        let blob = Blob::new(bytes.as_ref());
        let img = Image::builder().bytes(blob).build();
        req.image(img).send().await
    }

    pub fn normalize_category(input: &str) -> ModerationCategories {
        match input {
            "Explicit Nudity" => ModerationCategories::ExplicitNudity,
            "Suggestive" => ModerationCategories::Suggestive,
            "Violence" => ModerationCategories::Violence,
            "Visually Disturbing" => ModerationCategories::VisuallyDisturbing,
            "Rude" => ModerationCategories::Rude,
            "Drugs" => ModerationCategories::Drugs,
            "Tobacco" => ModerationCategories::Tobacco,
            "Alcohol" => ModerationCategories::Alcohol,
            "Gambling" => ModerationCategories::Gambling,
            "Hate" => ModerationCategories::Hate,
            _ => {
                error!("Unknown moderation category encountered, cat={}", input);
                ModerationCategories::Unknown
            }
        }
    }

    pub fn new(aws_region: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID key not set");
        env::var("AWS_SECRET_ACCESS_KEY").expect("AWS_SECRET_ACCESS_KEY key not set");
        Ok(Rekognition {
            region: aws_region.to_string(),
        })
    }
}
