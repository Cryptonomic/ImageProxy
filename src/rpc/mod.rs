pub mod requests;
pub mod responses;

use std::sync::Arc;

use hyper::Body;
use hyper::Response;
use log::{error, info, warn};
use uuid::Uuid;

use crate::{
    document::Document,
    metrics,
    moderation::{ModerationService, SupportedMimeTypes},
    proxy::Proxy,
};

use requests::*;
use responses::*;

/// rpc version information
pub static VERSION: &str = "1.0.0";

pub struct Methods {}

impl Methods {
    pub async fn fetch(
        proxy: Arc<Proxy>,
        req_id: &Uuid,
        params: &FetchRequestParams,
    ) -> Result<Response<Body>, StatusCodes> {
        info!(
            "New document fetch request, id={}, force={}, url={}",
            req_id, params.force, params.url
        );
        metrics::API_REQUESTS_FETCH.inc();

        // If forced, fetch document and return
        if params.force {
            metrics::DOCUMENTS_FORCED.inc();
            return Document::fetch(&proxy.config, req_id, &params.url)
                .await
                .map(|d| d.to_response());
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
            .unwrap_or(Vec::new());

        if cached_results.len() > 0 {
            if cached_results.len() > 1 {
                warn!("Found more than one cache results for id={}", req_id);
            }
            let r = &cached_results[0];
            metrics::CACHE_HITS.inc();
            info!(
                "Found cached results for id={}, blocked={}, categories:{:?}, provider:{:?}",
                req_id, r.blocked, r.categories, r.provider
            );
            // Send an appropriate response if moderation indicates content is blocked
            if r.blocked {
                Ok(FetchResponse::to_response(
                    StatusCodes::DocumentBlocked,
                    r.categories.clone(),
                ))
            } else {
                Document::fetch(&proxy.config, req_id, &params.url)
                    .await
                    .map(|d| d.to_response())
            }
        } else {
            metrics::CACHE_MISS.inc();
            info!("No cached results found for id={}", req_id);

            // Moderate and update the db
            let document = Document::fetch(&proxy.config, req_id, &params.url).await?;
            let document_type = SupportedMimeTypes::from_str(&document.content_type);

            if document_type == SupportedMimeTypes::Unsupported {
                return Ok(FetchResponse::to_response(
                    StatusCodes::UnsupportedImageType,
                    Vec::new(),
                ));
            }

            let max_document_size = proxy.moderation_provider.max_document_size();
            let supported_types = proxy.moderation_provider.supported_types();

            metrics::MODERATION_REQUESTS.inc();

            // Resize the image if required or reformat to png if required
            let mr = if document.content_length >= max_document_size
                || !supported_types.contains(&document_type)
            {
                let resized_doc = document.resize_image(document_type, max_document_size)?;
                proxy.moderation_provider.moderate(&resized_doc).await?
            } else {
                proxy.moderation_provider.moderate(&document).await?
            };

            let blocked = mr.categories.len() > 0;
            match proxy
                .database
                .add_moderation_result(&params.url, mr.provider, blocked, &mr.categories)
                .await
            {
                Ok(_) => info!("Database updated for id={}", req_id),
                Err(e) => error!("Database not updated for id={}, reason={}", req_id, e),
            }

            if blocked {
                metrics::DOCUMENTS_BLOCKED.inc();
                Ok(FetchResponse::to_response(
                    StatusCodes::DocumentBlocked,
                    mr.categories.clone(),
                ))
            } else {
                Ok(document.to_response())
            }
        }
    }

    pub async fn describe(
        proxy: Arc<Proxy>,
        req_id: &Uuid,
        params: &DescribeRequestParams,
    ) -> Result<Response<Body>, StatusCodes> {
        metrics::API_REQUESTS_DESCRIBE.inc();
        info!(
            "New describe request, id={}, urls={:?}",
            req_id, params.urls
        );

        let is_wildcard = "*" == params.urls.get(0).unwrap_or(&String::from(""));
        let from_db = match is_wildcard {
            true => proxy.database.get_all_moderation_result().await,
            _ => proxy.database.get_moderation_result(&params.urls).await,
        };
        match from_db {
            Ok(results) => {
                info!("Fetched results for id={}, rows={}", req_id, results.len());
                let urls = match is_wildcard {
                    true => results.iter().map(|r| r.url.clone()).collect(),
                    _ => params.urls.clone(),
                };
                let describe_results: Vec<DescribeResult> = urls
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
                                status: status,
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
                    StatusCodes::Ok,
                    describe_results,
                ))
            }
            Err(e) => {
                error!("Error querying database for id={}, reason={}", req_id, e);
                Err(StatusCodes::InternalError)
            }
        }
    }

    pub async fn report(
        proxy: Arc<Proxy>,
        req_id: &Uuid,
        params: &ReportRequestParams,
    ) -> Result<Response<Body>, StatusCodes> {
        metrics::API_REQUESTS_REPORT.inc();
        info!("New report request, id={}, url={}", req_id, params.url);
        match proxy
            .database
            .add_report(req_id, &params.url, &params.categories)
            .await
        {
            Ok(_) => Ok(ReportResponse::to_response(
                StatusCodes::Ok,
                &params.url,
                req_id,
            )),
            Err(e) => {
                error!("Database not updated for id={}, reason={}", req_id, e);
                Err(StatusCodes::InternalError)
            }
        }
    }

    pub async fn describe_report(
        proxy: Arc<Proxy>,
        req_id: &Uuid,
    ) -> Result<Response<Body>, StatusCodes> {
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
                    StatusCodes::Ok,
                    results,
                ))
            }
            Err(e) => {
                error!("Database not updated for id={}, reason={}", req_id, e);
                Err(StatusCodes::InternalError)
            }
        }
    }
}
