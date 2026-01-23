//! Image processing operations.

use std::path::Path;

use clap::ValueEnum;
use image::{DynamicImage, GenericImageView};

use crate::error::{Result, SdError};

/// Strategy for resizing images to match key dimensions.
#[derive(Debug, Clone, Copy, Default, ValueEnum, PartialEq, Eq)]
pub enum ResizeStrategy {
    /// Fit within key, maintain aspect ratio (may have black bars).
    #[default]
    Fit,
    /// Fill key, maintain aspect ratio (may crop).
    Fill,
    /// Stretch to fill (may distort).
    Stretch,
}

/// Load an image and resize it according to the specified strategy.
///
/// # Arguments
///
/// * `path` - Path to the image file.
/// * `width` - Target width.
/// * `height` - Target height.
/// * `strategy` - Resize strategy.
///
/// # Errors
///
/// Returns an error if the image cannot be loaded.
pub fn load_and_resize(
    path: &Path,
    width: u32,
    height: u32,
    strategy: ResizeStrategy,
) -> Result<DynamicImage> {
    if !path.exists() {
        return Err(SdError::ImageNotFound {
            path: path.display().to_string(),
        });
    }

    let img = image::open(path).map_err(|e| SdError::ImageProcessing(e.to_string()))?;

    let filter = image::imageops::FilterType::Lanczos3;

    let resized = match strategy {
        ResizeStrategy::Fit => {
            // resize() maintains aspect ratio and fits within bounds
            // We might need to pad it to fill the area if we want strict output size,
            // but usually returning a smaller image is fine for display drivers
            // that center it, OR we should pad with black.
            // The elgato crate expects exact dimensions?
            // "The image will be loaded, converted, and resized to match the device's key dimensions."
            // If we return a smaller image, set_button_image might fail or behave oddly.
            // Let's create a black canvas and paste the resized image on top.
            let resized = img.resize(width, height, filter).to_rgb8();
            let mut canvas = image::RgbImage::new(width, height);
            // Default is black (0,0,0)

            // Center the image
            let (rw, rh) = resized.dimensions();
            let x = (width - rw) / 2;
            let y = (height - rh) / 2;

            image::imageops::overlay(&mut canvas, &resized, x.into(), y.into());
            image::DynamicImage::ImageRgb8(canvas)
        }
        ResizeStrategy::Fill => {
            // resize_to_fill() maintains aspect ratio and fills bounds (cropping)
            img.resize_to_fill(width, height, filter)
        }
        ResizeStrategy::Stretch => {
            // resize_exact() ignores aspect ratio
            img.resize_exact(width, height, filter)
        }
    };

    Ok(resized)
}
