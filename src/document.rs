extern crate base64;
extern crate hyper;

use std::io::Cursor;

use crate::moderation::SupportedMimeTypes;
use crate::rpc::error::Errors;

use base64::encode;
use hyper::{body::Bytes, Body, Response};

use image::{self, GenericImageView};
use image::DynamicImage;
use image::ImageFormat;
use log::{debug, error};
use uuid::Uuid;

pub struct Document {
    pub id: Uuid,
    pub content_type: String,
    pub content_length: u64,
    pub bytes: Bytes,
}

impl Document {
    fn load_image(&self, image_type: SupportedMimeTypes) -> Result<DynamicImage, Errors> {
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
            Err(Errors::InternalError)
        })
    }

    pub fn resize_image(
        &self,
        image_type: SupportedMimeTypes,
        max_size: u64,
    ) -> Result<Document, Errors> {
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
                Err(Errors::InternalError)
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

    pub fn to_url(&self) -> String {
        format!(
            "data:{};base64,{}",
            self.content_type,
            encode(self.bytes.to_vec())
        )
    }
}
