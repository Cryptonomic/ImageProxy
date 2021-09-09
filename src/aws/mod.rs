mod messages;

use async_trait::async_trait;
use log::{debug, error};

use messages::RekognitionResponse;

use aws_sdk_rekognition::model::Image;
use aws_sdk_rekognition::{Blob, Client as ClientRekognition, Region};

use crate::{
    document::Document,
    moderation::{ModerationProvider, ModerationResponse, ModerationService, SupportedMimeTypes},
    rpc::error::Errors,
};

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
                let labels = result.get_labels();
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
    ) -> Result<RekognitionResponse, Box<dyn std::error::Error + Send + Sync>> {
        let region = Region::new(self.region.clone());
        let shared_config = aws_config::from_env().region(region).load().await;
        let client = ClientRekognition::new(&shared_config);
        let req = client.detect_moderation_labels();
        let blob = Blob::new(bytes.as_ref()); //.to_vec());
        let img = Image::builder().bytes(blob).build();
        let response = req.image(img).send().await;
        match response {
            Ok(output) => {
                let rekognition_repsonse: RekognitionResponse = output.into();
                Ok(rekognition_repsonse)
            }
            Err(e) => Err(Box::new(e)),
        }
    }

    pub fn new(aws_region: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Rekognition {
            region: aws_region.to_string(),
        })
    }
}
