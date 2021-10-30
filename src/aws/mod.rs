pub mod errors;
mod messages;
pub mod s3;

use crate::aws::errors::AwsError;

use std::sync::Arc;

use async_trait::async_trait;
use log::{debug, error};

use messages::RekognitionResponse;

use aws_sdk_rekognition::model::{Image, NotificationChannel, S3Object, Video, VideoJobStatus};

use aws_sdk_rekognition::{Blob, Client as ClientRekognition, Region};

use crate::{
    config::VideoConfig,
    document::Document,
    moderation::{ModerationProvider, ModerationResponse, ModerationService, SupportedMimeTypes},
    rpc::error::Errors,
};

use messages::Label;

use tokio::sync::Semaphore;
use tokio::time::{self, Duration};

pub struct Rekognition {
    pub region: String,
    pub rekognition_permits: Arc<Semaphore>,
    pub s3_permits: Arc<Semaphore>,
    pub video_config: Option<VideoConfig>,
    pub client: Option<ClientRekognition>,
}

#[async_trait]
impl ModerationProvider for Rekognition {
    async fn moderate(
        &self,
        document: &Document,
    ) -> Result<crate::moderation::ModerationResponse, Errors> {
        debug!("New Rekognition request");

        match self.get_moderation_labels(document).await {
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
        vec![
            SupportedMimeTypes::ImageJpeg,
            SupportedMimeTypes::ImagePng,
            SupportedMimeTypes::VideoMp4,
            SupportedMimeTypes::VideoMov,
        ]
    }

    fn max_document_size(&self, document_type: &SupportedMimeTypes) -> u64 {
        match document_type {
            SupportedMimeTypes::VideoMp4 => 10737418240,
            _ => 5242880,
        }
        // As per AWS documentation, 5 MB binary limit for images
        // but 15 mb for images added to bucket . 10gb for videos in bucket
    }
}

impl Rekognition {
    pub async fn start_video_moderation(
        &self,
        client: &ClientRekognition,
        channel: NotificationChannel,
        bucket: &str,
        key: &str,
    ) -> Result<String, AwsError> {
        let _permit = self.rekognition_permits.clone().acquire_owned().await?;

        debug!("starting video moderation ");

        let obj = S3Object::builder().bucket(bucket).name(key).build();
        let video = Video::builder().s3_object(obj).build();

        let r = client
            .start_content_moderation()
            .notification_channel(channel)
            .job_tag("Moderation")
            .video(video)
            .send()
            .await?; //.NotificationChannel(channel).

        let job_id = match r.job_id {
            Some(j) => j,
            None => {
                return Err("got an empty job id from rekognition".into());
            }
        }; //r.job_id.unwrap_or("".into());
        Ok(job_id)
    }

    // TODO : Add support for nextToken

    pub async fn get_moderation_results(
        &self,
        client: &ClientRekognition,
        job_id: String,
    ) -> Result<RekognitionResponse, AwsError> {
        let mut backoff: u64 = 1;
        let mut labels: Option<Vec<Label>> = None;

        debug!("waiting for moderation results after recieving job id ");

        loop {
            let _permit = self.rekognition_permits.clone().acquire_owned().await?;
            let mut pagination_token: Option<String> = None;

            let r = client
                .get_content_moderation()
                .job_id(job_id.clone())
                .set_next_token(pagination_token)
                .send()
                .await?;

            let model_ver = r
                .moderation_model_version
                .clone()
                .unwrap_or_else(|| "".into());

            match r.job_status {
                Some(VideoJobStatus::Succeeded) => {
                    pagination_token = r.next_token.clone();

                    if labels.is_none() {
                        labels = Label::get_labels_video(r);
                    } else {
                        labels = labels.map(|mut l| {
                            let lab: Option<Vec<Label>> = Label::get_labels_video(r);
                            match lab {
                                Some(lab) => {
                                    l.extend(lab);
                                    l
                                }
                                _ => l,
                            }
                        });
                    }

                    if pagination_token.is_none() {
                        return Ok(RekognitionResponse {
                            ModerationLabels: labels.unwrap_or_default(),
                            ModerationModelVersion: model_ver,
                        });
                    }
                }
                Some(VideoJobStatus::InProgress) => {
                    time::sleep(Duration::from_secs(backoff)).await;
                    backoff *= 2;
                }
                Some(status) => {
                    return Err(status.into());
                }
                _ => return Err("unknown error from rekonition , unknown VideoJob status".into()),
            }
        }
    }

    pub async fn get_moderation_labels(
        &self,
        document: &Document,
    ) -> Result<RekognitionResponse, AwsError> {
        let bytes: &hyper::body::Bytes = &document.bytes;
        if document.is_video() {
            if self.video_config.is_none() {
                Err("aws video config missing".into())
            } else {
                // set up region a shared config
                let bucket = &self.video_config.as_ref().unwrap().bucket;
                let sns_topic_arn = &self.video_config.as_ref().unwrap().sns_topic_arn;
                let s3_permits = &self.video_config.as_ref().unwrap().s3_jobs;

                let role_arn = &self.video_config.as_ref().unwrap().role_arn;

                let region = aws_sdk_s3::Region::new(self.region.as_str().to_owned());
                //we need this future to setup client , hence we can't set up the client in new ,as new is
                //not async

                let shared_config = aws_config::from_env().region(region).load().await;

                use crate::aws::s3::S3;
                let s3 = S3::new(&self.region.as_str().to_owned(), bucket, s3_permits).await;

                s3.add_to_bucket(document).await?;

                let channel = NotificationChannel::builder()
                    .sns_topic_arn(sns_topic_arn)
                    .role_arn(role_arn)
                    .build();

                // set up rekognition client
                let client_rekognition = ClientRekognition::new(&shared_config);

                // start video moderation and get job id
                let job_id = self
                    .start_video_moderation(&client_rekognition, channel, bucket, &document.url)
                    .await?;

                // get reuslts
                let response = self
                    .get_moderation_results(&client_rekognition, job_id)
                    .await?;

                Ok(response)
            }
        } else if document.is_image() {
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
                Err(e) => Err(e.to_string().into()),
            }
        } else {
            Err("document neither image or video".into())
        }
    }

    pub fn new(
        aws_region: &str,
        s3_jobs: &usize,
        rekognition_jobs: &usize,
        video_config: &Option<VideoConfig>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Rekognition {
            region: aws_region.to_string(),
            rekognition_permits: Arc::new(Semaphore::new(*rekognition_jobs)),
            s3_permits: Arc::new(Semaphore::new(*s3_jobs)),
            video_config: video_config.clone(),
            client: None,
        })
    }
}
