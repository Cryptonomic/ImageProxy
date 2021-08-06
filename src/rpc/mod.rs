pub mod error;
pub mod requests;
pub mod responses;

use std::sync::Arc;

use hyper::Body;
use hyper::Response;
use log::debug;
use log::{error, info, warn};
use uuid::Uuid;

use crate::document::Document;
use crate::moderation::ModerationResponse;
use crate::utils::sha512;
use crate::{
    metrics,
    moderation::{ModerationService, SupportedMimeTypes},
    proxy::Proxy,
    rpc::error::Errors,
};

use requests::*;
use responses::*;

/// rpc version information
pub static VERSION: &str = "1.0.0";

pub struct Methods;

impl Methods {
    async fn fetch_document(
        proxy: Arc<Proxy>,
        req_id: &Uuid,
        url: &str,
    ) -> Result<Arc<Document>, Errors> {
        if let Some(cache) = &proxy.cache {
            let cache_key = sha512(url.as_bytes());
            if let Some(document) = cache.get(&cache_key) {
                debug!("Fetched document from cache, url:{}", url);
                Ok(document)
            } else {
                let document = Arc::new(proxy.http_client.fetch(req_id, url).await?);
                debug!("Inserted document into cache, url:{}", url);
                cache.put(cache_key, document.clone());
                Ok(document)
            }
        } else {
            let document = proxy.http_client.fetch(req_id, url).await?;
            Ok(Arc::new(document))
        }
    }

    pub async fn fetch(
        proxy: Arc<Proxy>,
        req_id: &Uuid,
        params: &FetchRequestParams,
    ) -> Result<Response<Body>, Errors> {
        info!(
            "New document fetch request, id={}, force={}, url={}",
            req_id, params.force, params.url
        );

        // If forced, fetch document and return
        if params.force {
            metrics::DOCUMENT.with_label_values(&["forced"]).inc();

            let document = Methods::fetch_document(proxy.clone(), req_id, &params.url).await?;
            let document_type = SupportedMimeTypes::from_string(&document.content_type);

            if document_type == SupportedMimeTypes::Unsupported {
                return Ok(Errors::UnsupportedImageType.to_response(req_id));
            }
            metrics::TRAFFIC
                .with_label_values(&["served"])
                .inc_by(document.bytes.len() as u64);
            return match &params.response_type {
                ResponseType::Raw => Ok(document.to_response()),
                ResponseType::Json => Ok(FetchResponse::to_response(
                    RpcStatus::Ok,
                    ModerationStatus::Allowed,
                    Vec::new(),
                    Some(document.to_url()),
                    req_id,
                )),
            };
        }

        let urls = vec![params.url.clone()];
        // Check the database for prior results
        let cached_results = proxy
            .database
            .get_moderation_result(&urls)
            .await
            .map_err(|e| {
                error!("Error querying database for id={}, reason={}", req_id, e);
                e
            })
            .unwrap_or_default();

        if !cached_results.is_empty() {
            if cached_results.len() > 1 {
                warn!("Found more than one cache results for id={}", req_id);
            }
            let r = &cached_results[0];
            metrics::MODERATION.with_label_values(&["cache_hit"]).inc();
            info!(
                "Found cached results for id={}, blocked={}, categories:{:?}, provider:{:?}",
                req_id, r.blocked, r.categories, r.provider
            );
            // Send an appropriate response if moderation indicates content is blocked
            if r.blocked {
                Ok(FetchResponse::to_response(
                    RpcStatus::Ok,
                    ModerationStatus::Blocked,
                    r.categories.clone(),
                    None,
                    req_id,
                ))
            } else {
                let document = Methods::fetch_document(proxy.clone(), req_id, &params.url).await?;
                metrics::TRAFFIC
                    .with_label_values(&["served"])
                    .inc_by(document.bytes.len() as u64);
                match params.response_type {
                    ResponseType::Raw => Ok(document.to_response()),
                    ResponseType::Json => Ok(FetchResponse::to_response(
                        RpcStatus::Ok,
                        ModerationStatus::Allowed,
                        Vec::new(),
                        Some(document.to_url()),
                        req_id,
                    )),
                }
            }
        } else {
            metrics::MODERATION.with_label_values(&["cache_miss"]).inc();
            info!("No cached results found for id={}", req_id);

            // Moderate and update the db
            let document = Methods::fetch_document(proxy.clone(), req_id, &params.url).await?;

            let document_type = SupportedMimeTypes::from_string(&document.content_type);
            if document_type == SupportedMimeTypes::Unsupported {
                return Ok(Errors::UnsupportedImageType.to_response(req_id));
            }

            let max_document_size = proxy.moderation_provider.max_document_size();
            let supported_types = proxy.moderation_provider.supported_types();

            metrics::MODERATION.with_label_values(&["requests"]).inc();

            // Resize the image if required or reformat to png if required
            let formatted: Result<ModerationResponse, Errors> = if document.bytes.len() as u64
                >= max_document_size
                || !supported_types.contains(&document_type)
            {
                let resized_doc = document.resize_image(document_type, max_document_size)?;
                proxy.moderation_provider.moderate(&resized_doc).await
            } else {
                proxy.moderation_provider.moderate(&document).await
            };

            metrics::TRAFFIC
                .with_label_values(&["moderated"])
                .inc_by(document.bytes.len() as u64);

            match formatted {
                Ok(mr) => {
                    let blocked = !mr.categories.is_empty();
                    match proxy
                        .database
                        .add_moderation_result(&params.url, mr.provider, blocked, &mr.categories)
                        .await
                    {
                        Ok(_) => info!("Database updated for id={}", req_id),
                        Err(e) => {
                            error!("Database not updated for id={}, reason={}", req_id, e)
                        }
                    }

                    if blocked {
                        metrics::DOCUMENT.with_label_values(&["blocked"]).inc();
                        Ok(FetchResponse::to_response(
                            RpcStatus::Ok,
                            ModerationStatus::Blocked,
                            mr.categories.clone(),
                            None,
                            req_id,
                        ))
                    } else {
                        metrics::TRAFFIC
                            .with_label_values(&["served"])
                            .inc_by(document.bytes.len() as u64);
                        match params.response_type {
                            ResponseType::Raw => Ok(document.to_response()),
                            ResponseType::Json => Ok(FetchResponse::to_response(
                                RpcStatus::Ok,
                                ModerationStatus::Allowed,
                                Vec::new(),
                                Some(document.to_url()),
                                req_id,
                            )),
                        }
                    }
                }
                Err(e) => Ok(e.to_response(req_id)),
            }
        }
    }

    pub async fn describe(
        proxy: Arc<Proxy>,
        req_id: &Uuid,
        params: &DescribeRequestParams,
    ) -> Result<Response<Body>, Errors> {
        info!(
            "New describe request, id={}, urls={:?}",
            req_id, params.urls
        );
        match proxy.database.get_moderation_result(&params.urls).await {
            Ok(results) => {
                info!("Fetched results for id={}, rows={}", req_id, results.len());
                let describe_results: Vec<DescribeResult> = params
                    .urls
                    .iter()
                    .map(|url| match results.iter().find(|r| r.url.eq(url)) {
                        Some(res) => {
                            let status = if res.blocked {
                                DocumentStatus::Blocked
                            } else {
                                DocumentStatus::Allowed
                            };
                            DescribeResult {
                                url: url.clone(),
                                status,
                                categories: res.categories.clone(),
                                provider: res.provider.clone(),
                            }
                        }
                        None => DescribeResult {
                            url: url.clone(),
                            status: DocumentStatus::NeverSeen,
                            categories: Vec::new(),
                            provider: ModerationService::None,
                        },
                    })
                    .collect();
                Ok(DescribeResponse::to_response(
                    RpcStatus::Ok,
                    describe_results,
                    req_id,
                    &proxy.config,
                ))
            }
            Err(e) => {
                error!("Error querying database for id={}, reason={}", req_id, e);
                Err(Errors::InternalError)
            }
        }
    }

    pub async fn report(
        proxy: Arc<Proxy>,
        req_id: &Uuid,
        params: &ReportRequestParams,
    ) -> Result<Response<Body>, Errors> {
        info!("New report request, id={}, url={}", req_id, params.url);
        match proxy
            .database
            .add_report(req_id, &params.url, &params.categories)
            .await
        {
            Ok(_) => Ok(ReportResponse::to_response(
                RpcStatus::Ok,
                &params.url,
                req_id,
                &proxy.config,
            )),
            Err(e) => {
                error!("Database not updated for id={}, reason={}", req_id, e);
                Err(Errors::InternalError)
            }
        }
    }

    pub async fn describe_report(
        proxy: Arc<Proxy>,
        req_id: &Uuid,
    ) -> Result<Response<Body>, Errors> {
        info!("New report describe request, id={}", req_id);
        match proxy.database.get_reports().await {
            Ok(rows) => {
                let results: Vec<ReportDescribeResult> = rows
                    .iter()
                    .map(|r| ReportDescribeResult {
                        id: r.id.clone(),
                        url: r.url.clone(),
                        categories: r.categories.clone(),
                        updated_at: r.updated_at.to_string(),
                    })
                    .collect();
                Ok(ReportDescribeResponse::to_response(
                    RpcStatus::Ok,
                    results,
                    req_id,
                    &proxy.config,
                ))
            }
            Err(e) => {
                error!("Database not updated for id={}, reason={}", req_id, e);
                Err(Errors::InternalError)
            }
        }
    }
}
