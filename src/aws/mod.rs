extern crate base64;
extern crate crypto;

mod messages;
mod util;

use async_trait::async_trait;
use chrono::prelude::*;
use hyper::Client;
use hyper::{Body, Method, Request};
use hyper_tls::HttpsConnector;
use log::{debug, error};
use rustc_serialize::hex::ToHex;
use serde_json::json;
use std::env;

use messages::RekognitionResponse;
use util::{get_signature_key, hash, sign};

use crate::{
    document::Document,
    moderation::{ModerationProvider, ModerationResponse, ModerationService, SupportedMimeTypes},
    rpc::error::Errors,
};

pub struct Rekognition {
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
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
    fn get_host(&self) -> String {
        format!("rekognition.{}.amazonaws.com", self.region)
    }

    fn get_url(&self) -> String {
        format!("https://{}", self.get_host())
    }

    pub async fn get_moderation_labels(
        &self,
        bytes: &hyper::body::Bytes,
    ) -> Result<RekognitionResponse, Box<dyn std::error::Error + Send + Sync>> {
        let amz_target = "RekognitionService.DetectModerationLabels";
        let service = "rekognition";
        let content_type = "application/x-amz-json-1.1";
        let canonical_uri = "/";
        let canonical_querystring = "";
        let utc = Utc::now();
        let amz_date = utc.format("%Y%m%dT%H%M%SZ").to_string();
        let date_stamp = utc.format("%Y%m%d").to_string();
        let canonical_headers = format!(
            "content-type:{}\nhost:{}\nx-amz-date:{}\nx-amz-target:{}\n",
            content_type,
            self.get_host(),
            amz_date,
            amz_target
        );
        let signed_headers = "content-type;host;x-amz-date;x-amz-target";
        let algorithm = "AWS4-HMAC-SHA256";
        let credential_scope = format!("{}/{}/{}/aws4_request", date_stamp, self.region, service);
        let request_dict = json!({
            "Image": {
                "Bytes": base64::encode(bytes.to_vec()),
            },
            "MinConfidence": 50.0,
        });
        let request_dict_encoded = request_dict.to_string();
        let payload_hash = hash(&request_dict_encoded.to_string());
        let canonical_request = format!(
            "{}\n{}\n{}\n{}\n{}\n{}",
            "POST",
            canonical_uri,
            canonical_querystring,
            canonical_headers,
            signed_headers,
            payload_hash
        );
        //let canonical_request = format!("{}", canonical_request);
        let string_to_sign = format!(
            "{}\n{}\n{}\n{}",
            algorithm,
            amz_date,
            credential_scope,
            hash(&canonical_request)
        );
        let signing_key =
            get_signature_key(&self.secret_key, &date_stamp[..], &self.region, service);
        let signature = sign(&signing_key, string_to_sign.as_bytes());
        let authorization_header = format!(
            "{} Credential={}/{}, SignedHeaders={}, Signature={}",
            algorithm,
            self.access_key,
            credential_scope,
            signed_headers,
            signature.to_hex()
        );
        let req = Request::builder()
            .method(Method::POST)
            .uri(self.get_url())
            .header("Content-Type", content_type)
            .header("X-Amz-Date", amz_date)
            .header("X-Amz-Target", amz_target)
            .header("Authorization", authorization_header)
            .body(Body::from(request_dict_encoded))?;
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);

        match client.request(req).await {
            Ok(response) => {
                debug!("Rekognition response, status={}", response.status()); //TODO
                match hyper::body::to_bytes(response.into_body()).await {
                    Ok(bytes) => match serde_json::from_slice::<RekognitionResponse>(&bytes) {
                        Ok(r) => Ok(r),
                        Err(e) => Err(Box::new(e)),
                    },
                    Err(e) => Err(Box::new(e)),
                }
            }
            Err(e) => Err(Box::new(e)),
        }
    }

    pub fn new(aws_region: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Rekognition {
            region: aws_region.to_string(),
            access_key: env::var("AWS_ACCESS_KEY_ID").expect("AWS_ACCESS_KEY_ID key not set"),
            secret_key: env::var("AWS_SECRET_ACCESS_KEY")
                .expect("AWS_SECRET_ACCESS_KEY key not set"),
        })
    }
}
