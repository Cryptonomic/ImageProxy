use async_trait::async_trait;
use aws_sdk_s3::output::PutObjectOutput;
use chrono::Utc;
use log::{error, info};

use crate::{document::Document, rpc::error::Errors as RpcError, storage::*};

use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::aws::errors::*;

pub struct S3 {
    pub client: aws_sdk_s3::Client,
    pub region: String,
    pub bucket: String,
    pub permits: Arc<Semaphore>,
}

#[async_trait]
impl StorageProvider for S3 {
    async fn store(&self, document: &Document) -> Result<(), RpcError> {
        self.add_to_bucket(document).await?;

        Ok(())
    }
}

impl S3 {
    pub async fn add_to_bucket(&self, document: &Document) -> Result<PutObjectOutput, AwsError> {
        // Box<dyn std::error::Error + Send + Sync>> {
        let _permit = self.permits.clone().acquire_owned().await.unwrap();
        let response_time_start = Utc::now().timestamp_millis();
        let bytes: &hyper::body::Bytes = &document.bytes;

        info!("adding to bucket ");

        // TODO: add document meta data using tags

        let body = aws_sdk_s3::ByteStream::from(bytes.to_owned());
        let response = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(&document.url)
            .body(body)
            .send()
            .await
            .map_err(|e| {
                let e = AwsError::from(e);
                error!(
                    "document_id: {} , document_url : {} \n {}",
                    &document.id, &document.url, e
                );
                e
            })?;

        info!(
            "finished adding to bucket , time it took in ms   {}, speed bytes/milliseconds {} ",
            Utc::now().timestamp_millis() - response_time_start,
            (bytes.len() as i64) / (Utc::now().timestamp_millis() - response_time_start)
        );

        println!("Upload success. Version: {:?}", response.version_id);

        Ok(response)
    }

    pub async fn new(aws_region: &str, bucket: &str, permits: &usize) -> Self {
        let region = aws_sdk_s3::Region::new(aws_region.to_string());
        let shared_config = aws_config::from_env().region(region).load().await;

        //setup s3 client
        let client = aws_sdk_s3::Client::new(&shared_config);

        S3 {
            client,
            bucket: bucket.into(),
            region: aws_region.to_string(),
            permits: Arc::new(Semaphore::new(*permits)),
        }
    }
}
