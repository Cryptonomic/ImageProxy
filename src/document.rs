extern crate base64;
extern crate hyper;

use crate::rpc::error::Errors;
use crate::{cache::ByteSizeable, metrics};
use image::io::Reader as ImageReader;
use std::cmp::max;
use std::io::Cursor;

use base64::encode;
use hyper::body::Bytes;

use image::{DynamicImage, GenericImageView, ImageOutputFormat};
use log::{error, info, warn};
use uuid::Uuid;

// The X or Y resolution (depending on aspect ration) that is
// used as a first attempt target by image resizing
const NOMINAL_IMAGE_DIMENSION: u32 = 1024_u32;

// The minimum dimension for either X or Y
const MINIMUM_IMAGE_DIMENSION: u32 = 128_u32;

#[derive(Clone)]
pub struct Document {
    pub id: Uuid,
    pub content_type: String,
    pub content_length: u64,
    pub bytes: Bytes,
    pub url: String,
}

impl ByteSizeable for Document {
    fn size_in_bytes(&self) -> u64 {
        self.bytes.len() as u64
    }
}

impl Document {
    fn load_image(&self) -> Result<DynamicImage, Errors> {
        ImageReader::new(Cursor::new(&self.bytes))
            .with_guessed_format()
            .map_err(|e| {
                error!("Unable to open image, id={}, reason={}", self.id, e);
                Errors::ImageResizeError
            })
            .and_then(|r| {
                r.decode().map_err(|e| {
                    error!("Unable to open image, id={}, reason={}", self.id, e);
                    Errors::ImageResizeError
                })
            })
    }

    fn resize_parameters(x_dim: u32, y_dim: u32, target_dim: u32, min_dim: u32) -> (u32, u32) {
        let aspect_ratio = x_dim as f64 / y_dim as f64;
        if aspect_ratio > 1_f64 {
            // X is larger, set it to NOMINAL_IMAGE_DIMENSION
            let new_x_dim = target_dim as f64;
            let new_y_dim = new_x_dim / aspect_ratio;

            if new_y_dim < min_dim as f64 {
                let new_y_dim = min_dim as f64;
                let new_x_dim = aspect_ratio * new_y_dim;
                (new_x_dim.floor() as u32, new_y_dim.floor() as u32)
            } else {
                (new_x_dim.floor() as u32, new_y_dim.floor() as u32)
            }
        } else {
            // Y is larger, set it to NOMINAL_IMAGE_DIMENSION
            let new_y_dim = target_dim as f64;
            let new_x_dim = new_y_dim * aspect_ratio;

            if new_x_dim < min_dim as f64 {
                let new_x_dim = min_dim as f64;
                let new_y_dim = new_x_dim / aspect_ratio;
                (new_x_dim.floor() as u32, new_y_dim.floor() as u32)
            } else {
                (new_x_dim.floor() as u32, new_y_dim.floor() as u32)
            }
        }
    }

    fn resize(&self, img: DynamicImage, target_dim: u32, max_size: u64) -> Result<Vec<u8>, Errors> {
        metrics::IMAGE_RESIZE.with_label_values(&["request"]).inc();
        let (x_dim, y_dim) = img.dimensions();
        let (new_x_dim, new_y_dim) =
            Self::resize_parameters(x_dim, y_dim, target_dim, MINIMUM_IMAGE_DIMENSION);
        info!(
            "Image resizing, id={}, x={}, y={}, new_x={}, new_y={}",
            self.id, x_dim, y_dim, new_x_dim, new_y_dim
        );
        let new_img = img.resize(new_x_dim, new_y_dim, image::imageops::FilterType::Nearest);
        let mut cursor = Cursor::new(Vec::new());
        match new_img.write_to(&mut cursor, ImageOutputFormat::Png) {
            Ok(_) => {
                let bytes = cursor.into_inner();
                info!(
                    "Image resizing result, id={}, len={}, new_len={}",
                    self.id,
                    self.bytes.len(),
                    bytes.len()
                );
                if bytes.len() as u64 > max_size {
                    metrics::IMAGE_RESIZE.with_label_values(&["retry"]).inc();
                    warn!("Resizing did not reduce image size enough to fit max moderation size, id={}, max_size={}", self.id, max_size);
                    if target_dim / 2_u32 < MINIMUM_IMAGE_DIMENSION {
                        metrics::IMAGE_RESIZE
                            .with_label_values(&["dim_floor_hit"])
                            .inc();
                        warn!("Image dimension(s) is smaller than {} pixels but file size is greater than max moderation size, id={}, max_size={}", MINIMUM_IMAGE_DIMENSION, self.id, max_size );
                        Ok(bytes)
                    } else {
                        self.resize(new_img, target_dim / 2_u32, max_size)
                    }
                } else {
                    metrics::IMAGE_RESIZE.with_label_values(&["success"]).inc();
                    Ok(bytes)
                }
            }
            Err(e) => {
                metrics::IMAGE_RESIZE.with_label_values(&["failed"]).inc();
                error!(
                    "Error writing out image to buffer, id={}, reason={}",
                    self.id, e
                );
                Err(Errors::ImageResizeError)
            }
        }
    }

    pub fn resize_image(&self, max_size: u64) -> Result<Document, Errors> {
        info!(
            "Image info, id={}, len={}, type={}",
            self.id,
            self.bytes.len(),
            self.content_type
        );

        if (self.bytes.len() as u64) < max_size {
            info!(
                "Likely image format change, id={}, size={}, max_size={}",
                self.id,
                self.bytes.len(),
                max_size
            );
            metrics::IMAGE_RESIZE
                .with_label_values(&["format_change"])
                .inc();
        }

        let img = self.load_image()?;
        let bytes = self.resize(img, NOMINAL_IMAGE_DIMENSION, max_size)?;
        Ok(Document {
            id: self.id,
            content_length: bytes.len() as u64,
            content_type: String::from("image/png"),
            bytes: Bytes::copy_from_slice(bytes.as_slice()),
            url: self.url.clone(),
        })
    }

    pub fn to_url(&self) -> String {
        format!("data:{};base64,{}", self.content_type, encode(&self.bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImage, ImageOutputFormat, Rgba};
    use rand::Rng;

    const X_SIZE: u32 = 1600;
    const Y_SIZE: u32 = 1400;

    fn construct_image(x_dim: u32, y_dim: u32) -> Vec<u8> {
        let mut new_image = DynamicImage::new_rgba8(x_dim, y_dim);
        let mut rng = rand::thread_rng();

        for x in 0..x_dim - 1 {
            for y in 0..y_dim - 1 {
                let rgb: Rgba<u8> = Rgba([
                    rng.gen_range(0..255),
                    rng.gen_range(0..255),
                    rng.gen_range(0..255),
                    255,
                ]);
                new_image.put_pixel(x, y, rgb);
            }
        }
        let mut cursor = Cursor::new(Vec::new());

        new_image
            .write_to(&mut cursor, ImageOutputFormat::Png)
            .unwrap();

        cursor.into_inner()
    }

    fn construct_document(image_bytes: &[u8]) -> Document {
        let len = image_bytes.len() as u64;
        Document {
            id: Uuid::new_v4(),
            content_type: "image/png".to_string(),
            content_length: len,
            bytes: Bytes::copy_from_slice(image_bytes),
            url: "http://localhost.com/test.png".to_string(),
        }
    }

    #[test]
    fn test_to_url() {
        let bytes = "hello world".as_bytes();
        let document = construct_document(bytes);
        let encoded = document.to_url();
        assert_eq!(encoded.as_str(), "data:image/png;base64,aGVsbG8gd29ybGQ=");
    }

    #[test]
    fn test_resize_parameters() {
        let x_dim = 2048;
        let y_dim = 1500;
        let (x, y) = Document::resize_parameters(
            x_dim,
            y_dim,
            NOMINAL_IMAGE_DIMENSION,
            MINIMUM_IMAGE_DIMENSION,
        );
        assert_eq!(x, NOMINAL_IMAGE_DIMENSION);
        assert!(y < y_dim);

        let x_dim = 1500;
        let y_dim = 2048;
        let (x, y) = Document::resize_parameters(
            x_dim,
            y_dim,
            NOMINAL_IMAGE_DIMENSION,
            MINIMUM_IMAGE_DIMENSION,
        );
        assert!(x < x_dim);
        assert_eq!(y, NOMINAL_IMAGE_DIMENSION);

        // High X aspect ratio
        let x_dim = 2000;
        let y_dim = 200;
        let (x, y) = Document::resize_parameters(
            x_dim,
            y_dim,
            NOMINAL_IMAGE_DIMENSION,
            MINIMUM_IMAGE_DIMENSION,
        );
        assert!(x < x_dim);
        assert_eq!(y, MINIMUM_IMAGE_DIMENSION);

        // High Y aspect ratio
        let x_dim = 200;
        let y_dim = 2000;
        let (x, y) = Document::resize_parameters(
            x_dim,
            y_dim,
            NOMINAL_IMAGE_DIMENSION,
            MINIMUM_IMAGE_DIMENSION,
        );
        assert_eq!(x, MINIMUM_IMAGE_DIMENSION);
        assert!(y < y_dim);
    }

    #[test]
    fn test_image_functions() {
        let image_bytes = construct_image(X_SIZE, Y_SIZE);
        assert!(!image_bytes.is_empty());
        let document = construct_document(image_bytes.as_slice());

        // Check if image can be loaded
        let loaded_image = document.load_image();
        assert!(loaded_image.is_ok());
        let dimensions = loaded_image.unwrap().dimensions();
        assert_eq!(dimensions, (X_SIZE, Y_SIZE));

        // Check resize logic
        let max_size_5mb = 1024_u64 * 1024_u64 * 5_u64;

        // Resize required
        let new_document = document.resize_image(max_size_5mb);
        assert!(new_document.is_ok());
        let new_document = new_document.unwrap();
        assert!(new_document.bytes.len() < document.bytes.len());
        let loaded_image = new_document.load_image();
        assert!(loaded_image.is_ok());
        let dimensions = loaded_image.unwrap().dimensions();
        //TODO: Recheck why after img.resize is the y dimension of the image is off by -1
        assert_eq!(dimensions.0, NOMINAL_IMAGE_DIMENSION);
    }
}
