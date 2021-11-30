pub mod error;
pub mod requests;
pub mod responses;

use std::sync::Arc;

use hyper::Body;
use hyper::Response;
use log::{debug, error, info};
use uuid::Uuid;

use crate::document::Document;
use crate::utils::sha256;
use crate::{
    metrics,
    moderation::{ModerationService, SupportedMimeTypes},
    proxy::Context,
    rpc::error::Errors,
};

use requests::*;
use responses::*;

/// rpc version information
pub static VERSION: &str = "1.0.0";

async fn fetch_document(
    ctx: Arc<Context>,
    req_id: &Uuid,
    url: &str,
) -> Result<Arc<Document>, Errors> {
    let cache_key = sha256(url.as_bytes());
    let cached_doc = ctx
        .cache
        .as_ref()
        .map(|cache| {
            debug!("Fetched document from cache, url:{}", url);
            cache.get(&cache_key)
        })
        .flatten();

    if let Some(doc) = cached_doc {
        Ok(doc)
    } else {
        let document = Arc::new(ctx.http_client_provider.fetch(req_id, url).await?);
        if SupportedMimeTypes::from_string(&document.content_type)
            == SupportedMimeTypes::Unsupported
        {
            Err(Errors::UnsupportedImageType)
        } else {
            if let Some(cache) = &ctx.cache {
                debug!("Inserted document into cache, url:{}", url);
                cache.put(cache_key, document.clone());
            }
            Ok(document)
        }
    }
}

pub async fn fetch(
    ctx: Arc<Context>,
    req_id: &Uuid,
    params: &FetchRequestParams,
) -> Result<Response<Body>, Errors> {
    info!(
        "New fetch request, id={}, force={}, url={}",
        req_id, params.force, params.url
    );

    if params.force {
        metrics::DOCUMENT.with_label_values(&["forced"]).inc();
        info!("Document id={} has forced flag enabled.", req_id);
    }

    let urls = vec![params.url.clone()];
    let db_results = ctx
        .database
        .get_moderation_result(&urls)
        .await
        .map_err(|e| {
            error!("Error querying database for id={}, reason={}", req_id, e);
            Errors::InternalError
        })?;

    let (moderation_status, categories, document) = match db_results.get(0) {
        Some(result) => {
            info!(
                "Found cached results for id={}, blocked={}, categories:{:?}, provider:{:?}",
                req_id, result.blocked, result.categories, result.provider
            );
            let document = if !result.blocked || params.force {
                Some(fetch_document(ctx.clone(), req_id, &params.url).await?)
            } else {
                None
            };
            (result.blocked.into(), result.categories.clone(), document)
        }
        None => {
            metrics::MODERATION.with_label_values(&["cache_miss"]).inc();
            info!("No cached results found for id={}", req_id);
            let document = fetch_document(ctx.clone(), req_id, &params.url).await?;
            let max_document_size = ctx.moderation_provider.max_document_size();
            let supported_types = ctx.moderation_provider.supported_types();
            let document_type = SupportedMimeTypes::from_string(&document.content_type);

            metrics::MODERATION.with_label_values(&["requests"]).inc();

            // Resize the image if required or reformat to png if required
            let mod_response = if document.bytes.len() as u64 >= max_document_size
                || !supported_types.contains(&document_type)
            {
                let resized_doc = document.resize_image(max_document_size)?;
                ctx.moderation_provider.moderate(&resized_doc).await?
            } else {
                ctx.moderation_provider.moderate(&document).await?
            };

            metrics::TRAFFIC
                .with_label_values(&["moderated"])
                .inc_by(document.bytes.len() as u64);

            let blocked = !mod_response.categories.is_empty();
            let mod_status: ModerationStatus = blocked.into();

            let document = if !blocked || params.force {
                Some(document)
            } else {
                None
            };

            if blocked {
                metrics::DOCUMENT.with_label_values(&["blocked"]).inc();
            }

            let categories = mod_response.categories.clone();
            match ctx
                .database
                .add_moderation_result(
                    &params.url,
                    mod_response.provider,
                    blocked,
                    &mod_response.categories,
                )
                .await
            {
                Ok(_) => info!("Database updated for id={}", req_id),
                Err(e) => {
                    error!("Database not updated for id={}, reason={}", req_id, e)
                }
            };
            (mod_status, categories, document)
        }
    };

    Ok(FetchResponse::to_response(
        &params.response_type,
        document,
        moderation_status,
        categories,
        req_id,
        &ctx.config,
    ))
}

pub async fn describe(
    ctx: Arc<Context>,
    req_id: &Uuid,
    params: &DescribeRequestParams,
) -> Result<Response<Body>, Errors> {
    info!(
        "New describe request, id={}, urls={:?}",
        req_id, params.urls
    );
    match ctx.database.get_moderation_result(&params.urls).await {
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
                &ctx.config,
            ))
        }
        Err(e) => {
            error!("Error querying database for id={}, reason={}", req_id, e);
            Err(Errors::InternalError)
        }
    }
}

pub async fn report(
    ctx: Arc<Context>,
    req_id: &Uuid,
    params: &ReportRequestParams,
) -> Result<Response<Body>, Errors> {
    info!("New report request, id={}, url={}", req_id, params.url);
    match ctx
        .database
        .add_report(req_id, &params.url, &params.categories)
        .await
    {
        Ok(_) => Ok(ReportResponse::to_response(
            RpcStatus::Ok,
            &params.url,
            req_id,
            &ctx.config,
        )),
        Err(e) => {
            error!("Database not updated for id={}, reason={}", req_id, e);
            Err(Errors::InternalError)
        }
    }
}

pub async fn describe_report(ctx: Arc<Context>, req_id: &Uuid) -> Result<Response<Body>, Errors> {
    info!("New report describe request, id={}", req_id);
    match ctx.database.get_reports().await {
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
                &ctx.config,
            ))
        }
        Err(e) => {
            error!("Database not updated for id={}, reason={}", req_id, e);
            Err(Errors::InternalError)
        }
    }
}
