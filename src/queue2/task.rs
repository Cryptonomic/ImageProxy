use std::fs::ReadDir;
use std::sync::Arc;

use log::{error, info};
use serde::__private::de::Content;
use uuid::Uuid;

//use aws_types::credentials::Result;
use crate::aws::Rekognition;

use crate::document::Document;
use crate::moderation::ModerationProvider;
use crate::proxy::Context;
use crate::rpc::responses::ModerationStatus;

type GenericError = Box<dyn std::error::Error + Send + Sync>;

pub struct VideoModerationTask {
    state: VideoModerationState,
    pub url: String,
    req_id: Uuid,
    context: Arc<Context>,
    result: Option<Result<crate::moderation::ModerationResponse, crate::rpc::error::Errors>>,
    job_id: Option<String>,
}
impl VideoModerationTask {
  pub async fn new(
        context: Arc<Context>,
        req_id: Uuid,
        url: String,

    ) -> Self {
        Self {
            state: VideoModerationState::AddToBucket,
            url: url,
            context: context,
            req_id: req_id,
            result: None,
            job_id: None,
        }
    }

    pub fn is_active(&self) -> bool {
        match self.state {
            VideoModerationState::WaitingForResult => true,
            _ => false,
        }
    }

    pub async fn is_ready(&self) -> bool {
        match self.state {
            VideoModerationState::AddToBucket => true,
            _ => false,
        }
    }

    async fn set_to_active(&mut self) {
        self.state = VideoModerationState::WaitingForResult;
    }

    pub async fn complete(&mut self) {
        info!("task {} has begun",self.url);
        if self.is_ready().await {
            self.set_to_active().await;

            match crate::rpc::fetch_document(self.context.clone(), &self.req_id, &self.url).await {
                Ok(doc) => {
                    let res = self
                        .context
                        .moderation_provider
                        .moderate(doc.as_ref())
                        .await;
                    //self.result = Some(res);
                    match res {
                        Ok(m) => {
                            let blocked = !m.categories.is_empty();

                            let mod_status: ModerationStatus = blocked.into();

                            let categories = m.categories.clone();
                            match self
                                .context
                                .database
                                .add_moderation_result(
                                    &self.url,
                                    m.provider,
                                    blocked,
                                    &m.categories,
                                )
                                .await
                            {
                                Ok(_) => {
                                    self.state = VideoModerationState::GotResult;
                                    info!("Database updated for id={}", self.req_id)
                                }
                                Err(e) => {
                                    self.state = VideoModerationState::Failed;
                                    error!(
                                        "Database not updated for id={}, reason={}",
                                        self.req_id, e
                                    )
                                }
                            };
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    self.state = VideoModerationState::Failed;
                }
            }
        }
        info!("task {} has finished",self.url);
    }
}

enum VideoModerationState {
    AddToBucket,
    WaitingForResult,
    GotResult,
    Failed,
}
