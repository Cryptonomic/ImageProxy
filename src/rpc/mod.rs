pub mod error;
pub mod requests;
pub mod responses;

use std::sync::Arc;

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
    let cached_doc = ctx.cache.as_ref().and_then(|cache| {
        debug!("Checking document cache for document, url:{}", url);
        cache.get(&cache_key)
    });

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
) -> Result<ModerationResult, Errors> {
    info!(
        "New fetch request, id={}, force={}, url={}",
        req_id, params.force, params.url
    );

    if params.force {
        metrics::DOCUMENT.with_label_values(&["forced"]).inc();
        info!("Document id={} has forced flag enabled.", req_id);
    }

    let db_results = if let Some(result) = ctx.db_cache.get(&params.url) {
        info!(
            "Database moderation query skipped. Using cached results, id={}",
            req_id
        );
        vec![result]
    } else {
        info!("Querying database for moderation results, id={}", req_id);
        let results = ctx
            .database
            .get_moderation_result(&[params.url.clone()])
            .await
            .map_err(|e| {
                error!("Error querying database for id={}, reason={}", req_id, e);
                Errors::InternalError
            })?;
        results.iter().for_each(|r| {
            ctx.db_cache.insert(r.url.clone(), r.clone());
        });
        results
    };

    let (moderation_status, categories, document) = match db_results.get(0) {
        Some(result) => {
            metrics::MODERATION.with_label_values(&["cache_hit"]).inc();
            info!(
                "Found cached moderation results for id={}, blocked={}, categories:{:?}, provider:{:?}",
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
            info!("No cached moderation results found for id={}", req_id);
            let document = fetch_document(ctx.clone(), req_id, &params.url).await?;
            let max_document_size = ctx.moderation_provider.max_document_size();
            let supported_types = ctx.moderation_provider.supported_types();
            let document_type = SupportedMimeTypes::from_string(&document.content_type);

            metrics::MODERATION.with_label_values(&["requests"]).inc();

            info!("Submitting moderation request for id:{}", req_id);
            // Resize the image if required or reformat to png if required
            let mod_response = if document.bytes.len() as u64 >= max_document_size
                || !supported_types.contains(&document_type)
            {
                info!("Image resizing required, id={}", req_id);
                let resized_doc = document.resize_image(max_document_size)?;
                ctx.moderation_provider.moderate(&resized_doc).await?
            } else {
                ctx.moderation_provider.moderate(&document).await?
            };

            metrics::TRAFFIC
                .with_label_values(&["moderated"])
                .inc_by(document.bytes.len() as u64);

            mod_response.categories.iter().for_each(|c| {
                metrics::MODERATION_CATEGORIES
                    .with_label_values(&[&c.to_string()])
                    .inc()
            });

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

    //TODO: This section needs rework in version 2.0.0. See issue #83.
    let result = if params.force {
        ModerationResult {
            moderation_status: ModerationStatus::Allowed,
            categories: vec![],
            data: String::default(), //TODO: This smells, refactor away without breaking API
            document,
        }
    } else {
        ModerationResult {
            moderation_status,
            categories,
            data: String::default(), //TODO: This smells, refactor away without breaking API
            document,
        }
    };

    Ok(result)
}

pub async fn describe(
    ctx: Arc<Context>,
    req_id: &Uuid,
    params: &DescribeRequestParams,
) -> Result<Vec<DescribeResult>, Errors> {
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
            Ok(describe_results)
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
) -> Result<(), Errors> {
    info!("New report request, id={}, url={}", req_id, params.url);
    ctx.database
        .add_report(req_id, &params.url, &params.categories)
        .await
        .map_err(|e| {
            error!("Database not updated for id={}, reason={}", req_id, e);
            Errors::InternalError
        })
}

pub async fn describe_report(
    ctx: Arc<Context>,
    req_id: &Uuid,
) -> Result<Vec<ReportDescribeResult>, Errors> {
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
            Ok(results)
        }
        Err(e) => {
            error!("Database not updated for id={}, reason={}", req_id, e);
            Err(Errors::InternalError)
        }
    }
}

#[cfg(test)]
mod tests {
    use hyper::body::Bytes;
    use moka::sync::Cache as MokaCache;
    use uuid::Uuid;

    use crate::config::{Host, IpfsGatewayConfig};
    use crate::db::tests::DummyDatabase;
    use crate::dns::DummyDnsResolver;
    use crate::document::Document;
    use crate::http::filters::private_network::PrivateNetworkFilter;
    use crate::http::filters::UriFilter;
    use crate::http::tests::DummyHttpClient;
    use crate::http::HttpClientWrapper;
    use crate::moderation::tests::DummyModerationProvider;
    use crate::moderation::ModerationCategories;

    use crate::proxy::Context;
    use std::net::IpAddr;
    use std::sync::Arc;

    use super::*;

    const URL_SAFE_IMAGE: &str = "http://cryptonomic.tech/test.png";
    const URL_UNSAFE_IMAGE: &str = "http://cryptonomic.tech/drugs.png";
    const URL_404: &str = "http://cryptonomic.tech/404.png";

    fn construct_context(
        document: Option<Document>,
        categories: Option<Vec<ModerationCategories>>,
    ) -> Arc<Context> {
        let database = DummyDatabase::new();
        let mut moderation_provider = DummyModerationProvider::new();
        let mut http_client = DummyHttpClient::new();
        let ipfs_config = IpfsGatewayConfig {
            primary: Host {
                protocol: "http".to_string(),
                host: "localhost".to_string(),
                port: 1337,
                path: "/ipfs".to_string(),
            },
            fallback: None,
        };
        let ip: IpAddr = "8.8.8.8".parse().unwrap();
        let ip_vec = vec![ip];
        let dns_resolver = DummyDnsResolver {
            resolved_address: ip_vec,
        };
        let uri_filters: Vec<Box<dyn UriFilter + Send + Sync>> =
            vec![Box::new(PrivateNetworkFilter::new(Box::new(dns_resolver)))];

        if let Some(doc) = document {
            let url = doc.url.clone();
            http_client.set(&url.clone(), doc);
            if let Some(cats) = categories {
                moderation_provider.set(&url, cats)
            };
        }

        let http_client_provider =
            HttpClientWrapper::new(Box::new(http_client), ipfs_config, uri_filters);

        let context = Context {
            database: Box::new(database),
            moderation_provider: Box::new(moderation_provider),
            http_client_provider,
            cache: None,
            db_cache: Arc::new(MokaCache::new(10)),
        };

        Arc::new(context)
    }

    fn construct_document(url: &str) -> Document {
        let buffer = "Hello There";
        Document {
            id: Uuid::new_v4(),
            content_type: "image/png".to_string(),
            content_length: buffer.len() as u64,
            bytes: Bytes::from(buffer),
            url: url.to_string(),
        }
    }

    #[tokio::test]
    async fn test_fetch_document_ok() {
        let doc = construct_document(URL_SAFE_IMAGE);
        let context = construct_context(Some(doc), None);

        // Fetch an image that exists
        let document = fetch_document(context.clone(), &Uuid::new_v4(), URL_SAFE_IMAGE).await;
        assert!(document.is_ok());

        // Fetch an image that doesn't exist
        let document = fetch_document(context, &Uuid::new_v4(), URL_404).await;
        assert!(document.is_err());
    }

    #[tokio::test]
    async fn test_fetch_safe_image() {
        let doc = construct_document(URL_SAFE_IMAGE);
        let context = construct_context(Some(doc), None);

        let params = FetchRequestParams {
            url: URL_SAFE_IMAGE.to_string(),
            force: false,
            response_type: ResponseType::Json,
        };
        let result = fetch(context, &Uuid::new_v4(), &params).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.moderation_status, ModerationStatus::Allowed);
    }

    #[tokio::test]
    async fn test_fetch_unsafe_image() {
        let categories = vec![ModerationCategories::Drugs];
        let doc = construct_document(URL_UNSAFE_IMAGE);
        let context = construct_context(Some(doc), Some(categories));

        let params = FetchRequestParams {
            url: URL_UNSAFE_IMAGE.to_string(),
            force: false,
            response_type: ResponseType::Json,
        };
        let result = fetch(context, &Uuid::new_v4(), &params).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.moderation_status, ModerationStatus::Blocked);
        assert_eq!(result.categories.len(), 1);
        assert!(result.categories.contains(&ModerationCategories::Drugs));
        // no data should be returned
        assert!(result.document.is_none());
    }

    #[tokio::test]
    async fn test_fetch_unsafe_image_force() {
        let categories = vec![ModerationCategories::Drugs];
        let doc = construct_document(URL_UNSAFE_IMAGE);
        let context = construct_context(Some(doc), Some(categories));

        let params = FetchRequestParams {
            url: URL_UNSAFE_IMAGE.to_string(),
            force: true,
            response_type: ResponseType::Json,
        };
        let result = fetch(context, &Uuid::new_v4(), &params).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        //TODO: Uncomment when ticket #83 is implementd.
        // assert_eq!(result.moderation_status, ModerationStatus::Blocked);
        // assert_eq!(result.categories.len(), 1);
        // assert!(result.categories.contains(&ModerationCategories::Drugs));
        // data should be returned due to force = true flag
        assert!(result.document.is_some());
    }

    #[tokio::test]
    async fn test_describe() {
        let context = construct_context(None, None);
        let database = &context.database;
        // Insert results into the database
        let result = database
            .add_moderation_result(URL_SAFE_IMAGE, ModerationService::Aws, false, &[])
            .await;
        assert!(result.is_ok());

        let result = database
            .add_moderation_result(
                URL_UNSAFE_IMAGE,
                ModerationService::Aws,
                true,
                &[ModerationCategories::Drugs],
            )
            .await;
        assert!(result.is_ok());

        let params = DescribeRequestParams {
            urls: vec![
                URL_SAFE_IMAGE.to_string(),
                URL_UNSAFE_IMAGE.to_string(),
                URL_404.to_string(),
            ],
        };
        let result = describe(context, &Uuid::new_v4(), &params).await;
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.len(), 3);
        let result1 = &result[0];
        let result2 = &result[1];
        let result3 = &result[2];

        assert_eq!(result1.url, URL_SAFE_IMAGE);
        assert_eq!(result1.status, DocumentStatus::Allowed);
        assert_eq!(result1.provider, ModerationService::Aws);
        assert!(result1.categories.is_empty());

        assert_eq!(result2.url, URL_UNSAFE_IMAGE);
        assert_eq!(result2.status, DocumentStatus::Blocked);
        assert_eq!(result2.provider, ModerationService::Aws);
        assert_eq!(result2.categories.len(), 1);
        assert!(result2.categories.contains(&ModerationCategories::Drugs));

        assert_eq!(result3.url, URL_404);
        assert_eq!(result3.status, DocumentStatus::NeverSeen);
        assert_eq!(result3.provider, ModerationService::None);
        assert!(result3.categories.is_empty());
    }
}
