extern crate hyper;

use std::io::Cursor;

use crate::{
    config::{Configuration, Host},
    metrics,
    moderation::SupportedMimeTypes,
    rpc::errors::ImgProxyError,
};
use hyper::Client;
use hyper::{
    body::{to_bytes, Bytes},
    Body, Response, Uri,
};
use hyper_tls::HttpsConnector;
use image::DynamicImage;
use image::ImageFormat;
use image::{self, GenericImageView};
use log::{debug, error, info};
use uuid::Uuid;

pub struct Document {
    pub id: Uuid,
    pub content_type: String,
    pub content_length: u64,
    pub bytes: Bytes,
}

impl Document {
    fn to_uri(ipfs_config: &Host, url: &String) -> Result<Uri, ImgProxyError> {
        match url {
            u if u.starts_with("http://") | u.starts_with("https://") => {
                u.parse::<Uri>().map_err(|e| {
                    error!("Error parsing url={}, reason={}", u, e);
                    ImgProxyError::InvalidUri
                })
            }
            u if u.starts_with("ipfs://") => {
                let ipfs_path = url.strip_prefix("ipfs://").unwrap_or_default();
                let gateway_url = format!(
                    "{}://{}:{}{}/{}",
                    ipfs_config.protocol,
                    ipfs_config.host,
                    ipfs_config.port,
                    ipfs_config.path,
                    ipfs_path
                );
                debug!("Ipfs gateway path: {}", gateway_url);
                gateway_url.parse::<Uri>().map_err(|e| {
                    error!("Error parsing url={}, reason={}", u, e);
                    ImgProxyError::InvalidUri
                })
            }
            _ => Err(ImgProxyError::UnsupportedUriScheme),
        }
    }

    fn load_image(&self, image_type: SupportedMimeTypes) -> Result<DynamicImage, ImgProxyError> {
        let cursor = Cursor::new(&self.bytes);
        let img = match image_type {
            SupportedMimeTypes::ImageBmp => image::load(cursor, ImageFormat::Bmp),
            SupportedMimeTypes::ImageGif => image::load(cursor, ImageFormat::Gif),
            SupportedMimeTypes::ImageJpeg => image::load(cursor, ImageFormat::Jpeg),
            SupportedMimeTypes::ImagePng => image::load(cursor, ImageFormat::Png),
            SupportedMimeTypes::ImageTiff => image::load(cursor, ImageFormat::Tiff),
            SupportedMimeTypes::Unsupported => image::load(cursor, ImageFormat::Jpeg), //TODO
        };
        img.or_else(|e| {
            error!("Unable to open image, reason={}", e);
            Err(ImgProxyError::InternalError)
        })
    }

    pub fn resize_image(
        &self,
        image_type: SupportedMimeTypes,
        max_size: u64,
    ) -> Result<Document, ImgProxyError> {
        let img = self.load_image(image_type)?;
        let (x_dim, y_dim) = img.dimensions();
        let scale = self.content_length as f64 / max_size as f64;
        let scale_factor: u32 = 2_u32.pow(scale.max(0_f64) as u32);
        debug!("Image resize: scale={}, factor={}", scale, scale_factor);
        let (x_dim_new, y_dim_new) = (x_dim / scale_factor, y_dim / scale_factor);
        debug!(
            "Image resize: New dimensions x={}, y={}",
            x_dim_new, y_dim_new
        );
        let new_img = img.resize(x_dim_new, y_dim_new, image::imageops::FilterType::Nearest); //TODO this is expensive
        let mut bytes: Vec<u8> = Vec::new();
        match new_img.write_to(&mut bytes, image::ImageOutputFormat::Png) {
            Ok(_) => Ok(Document {
                id: self.id.clone(),
                content_length: bytes.len() as u64,
                content_type: String::from("image/png"),
                bytes: Bytes::copy_from_slice(bytes.as_slice()),
            }),
            Err(e) => {
                error!("Error writing out image to buffer, reason={}", e);
                Err(ImgProxyError::InternalError)
            }
        }
    }

    pub async fn fetch(
        config: &Configuration,
        req_id: &Uuid,
        url: &String,
    ) -> Result<Document, ImgProxyError> {
        info!("Fetching document for id:{}, url:{}", req_id, url);
        let uri = Document::to_uri(&config.ipfs, url)?;
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);
        match client.get(uri).await {
            Ok(response) => match response.status() {
                hyper::StatusCode::OK => {
                    let headers = response.headers().clone();
                    let content_length = headers
                        .get(hyper::header::CONTENT_LENGTH)
                        .map(|h| {
                            String::from_utf8(h.as_bytes().to_vec())
                                .map(|s| s.parse::<u64>().ok())
                                .ok()
                        })
                        .flatten()
                        .flatten()
                        .unwrap_or(0);
                    let content_type = headers
                        .get(hyper::header::CONTENT_TYPE)
                        .map(|h| String::from_utf8(h.as_bytes().to_vec()).ok())
                        .flatten()
                        .unwrap_or("".to_string());
                    let bytes = to_bytes(response.into_body()).await.map_err(|e| {
                        error!("Error retrieving document body, reason={}", e);
                        ImgProxyError::FetchFailed
                    })?;

                    info!(
                        "Document fetched for id={}, content_length={:?}, content_type={:?}",
                        req_id, content_length, content_type
                    );
                    metrics::DOCUMENTS_FETCHED.inc();
                    metrics::BYTES_FETCHED.inc_by(content_length as i64);
                    metrics::DOCUMENT_SIZE
                        .with_label_values(&["size_bytes"])
                        .observe(content_length as f64);

                    Ok(Document {
                        id: req_id.clone(),
                        content_type: content_type,
                        content_length: content_length,
                        bytes: bytes,
                    })
                }
                hyper::StatusCode::NOT_FOUND => {
                    metrics::DOCUMENTS_FETCHED_ERROR.inc();
                    error!("Document not found on remote, id={}", req_id);
                    Err(ImgProxyError::NotFound)
                }
                e => {
                    metrics::DOCUMENTS_FETCHED_ERROR.inc();
                    error!(
                        "Unable to fetch document, id={}, response_code={}",
                        req_id, e
                    );
                    Err(ImgProxyError::FetchFailed)
                }
            },
            Err(e) => {
                metrics::DOCUMENTS_FETCHED_ERROR.inc();
                error!("Unable to fetch document, id={}, reason={}", req_id, e);
                Err(ImgProxyError::FetchFailed)
            }
        }
    }

    pub fn to_response(&self) -> Response<Body> {
        Response::builder()
            .status(200)
            .header(hyper::header::CONTENT_TYPE, self.content_type.clone())
            .header(hyper::header::CONTENT_LENGTH, self.bytes.len())
            .body(Body::from(self.bytes.clone()))
            .unwrap_or_default()
    }
}
