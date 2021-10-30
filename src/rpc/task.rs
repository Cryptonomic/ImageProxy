use std::sync::Arc;

use log::{error, info};
use uuid::Uuid;

use crate::{proxy::Context, queue2::Task};

use async_trait::async_trait;

pub struct VideoModerationTask {
    pub url: String,
    req_id: Uuid,
    context: Arc<Context>,
}
impl VideoModerationTask {
    pub async fn new(context: Arc<Context>, req_id: Uuid, url: String) -> Self {
        Self {
            url,
            context,
            req_id,
        }
    }
}

#[async_trait]
impl Task for VideoModerationTask {
    fn get_id(&self) -> String {
        self.url.to_owned()
    }
    async fn complete(&mut self) {
        info!("task {} has begun", self.url);

        match crate::rpc::fetch_document(self.context.clone(), &self.req_id, &self.url).await {
            Ok(doc) => {
                let res = self
                    .context
                    .moderation_provider
                    .moderate(doc.as_ref())
                    .await;
                if let Ok(m) = res {
                    let blocked = !m.categories.is_empty();

                    match self
                        .context
                        .database
                        .add_moderation_result(&self.url, m.provider, blocked, &m.categories)
                        .await
                    {
                        Ok(_) => {
                            info!("Database updated for id={}", self.req_id)
                        }
                        Err(e) => {
                            error!("Database not updated for id={}, reason={}", self.req_id, e)
                        }
                    };
                }
            }
            Err(_e) => {}
        }
    }
}
