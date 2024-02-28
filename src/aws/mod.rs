use async_trait::async_trait;
use aws_config::Region;
use log::{debug, error, warn};
use std::env;

use crate::{
    document::Document,
    moderation::{
        ModerationCategories, ModerationProvider, ModerationResponse, ModerationService,
        SupportedMimeTypes,
    },
    rpc::error::Errors,
};

use aws_sdk_rekognition::{error::SdkError, operation::detect_moderation_labels::{DetectModerationLabelsError, DetectModerationLabelsOutput}, primitives::Blob, types::Image};
use aws_sdk_rekognition:: Client as ClientRekognition;

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
                debug!("Rekognition Result: {:?}", result);
                let labels = result.moderation_labels.unwrap_or_default();
                let mut labels: Vec<ModerationCategories> = labels
                    .into_iter()               
                    .filter(|l| (l.parent_name().is_none() || l.parent_name() == Some("")) && (l.name.is_some()))   // Only interested in top level labels
                    .map(|l| {                                                
                        let normalized_category = l.name().map(Rekognition::normalize_category).unwrap_or(ModerationCategories::Unknown);                        
                        if normalized_category == ModerationCategories::Unknown {
                            warn!("Label normalization failed for Rekognition: id={}, label_name={:?}, label_parent={:?}", document.id, l.name(), l.parent_name());
                        }
                        normalized_category             
                    })
                    .collect();

                labels.sort();
                labels.dedup();

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
                error!("Moderation failed, id={}, reason:{}", document.id, e);
                Err(Errors::ModerationFailed)
            }
        }
    }

    fn supported_types(&self) -> Vec<SupportedMimeTypes> {
        vec![SupportedMimeTypes::ImageJpeg, SupportedMimeTypes::ImagePng]
    }

    fn max_document_size(&self) -> u64 {
        // As per AWS documentation, 5 MB binary limit then scaled by
        // generous encoding margin
        (5242880_f64 / 1.5_f64).ceil() as u64
    }
}

impl Rekognition {
    pub async fn get_moderation_labels(
        &self,
        bytes: &hyper::body::Bytes,
    ) -> Result<DetectModerationLabelsOutput, SdkError<DetectModerationLabelsError, >> {
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
            // Model 7
            "Explicit" => ModerationCategories::ExplicitContent,
            "Rude Gestures" => ModerationCategories::Rude,
            "Drugs & Tobacco" => ModerationCategories::DrugsAndTobacco,
            "Non-Explicit Nudity of Intimate parts and Kissing" => ModerationCategories::Suggestive,
            "Swimwear or Underwear" => ModerationCategories::Suggestive,
            _ => ModerationCategories::Unknown,
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
