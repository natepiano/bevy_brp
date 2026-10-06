//! Captured target cropping, RGB conversion, and PNG encoding.

use bevy::image::TextureFormatPixelInfo;
use bevy::prelude::*;
use bevy::render::render_resource::Extent3d;
use bevy::render::render_resource::TextureDimension;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
use image::ExtendedColorType;
use image::ImageEncoder;
use image::ImageError;
use image::RgbImage;
use image::codecs::png::CompressionType;
use image::codecs::png::FilterType;
use image::codecs::png::PngEncoder;

pub(super) struct TargetRgbImage(RgbImage);

impl TargetRgbImage {
    /// Crops the captured rows first, so only the kept pixels go through the RGB conversion.
    pub(super) fn from_capture(image: Image, crop: Option<URect>) -> BrpResult<Self> {
        let actual_extent = URect::from_corners(UVec2::ZERO, image.size());
        if actual_extent.is_empty() {
            return Err(capture_error("Captured image has an empty extent"));
        }

        let requested_extent = crop.unwrap_or(actual_extent);
        let capture_extent = requested_extent.intersect(actual_extent);
        if capture_extent.is_empty() || capture_extent != requested_extent {
            return Err(capture_error(format!(
                "Captured extent {}x{} is smaller than the promised crop {}x{} at ({}, {})",
                actual_extent.width(),
                actual_extent.height(),
                requested_extent.width(),
                requested_extent.height(),
                requested_extent.min.x,
                requested_extent.min.y
            )));
        }

        let image = if capture_extent == actual_extent {
            image
        } else {
            crop_rows(&image, capture_extent)?
        };
        image
            .try_into_dynamic()
            .map(|dynamic_image| Self(dynamic_image.into_rgb8()))
            .map_err(|error| {
                capture_error(format!("Failed to convert captured image to RGB: {error}"))
            })
    }

    /// Encodes with png's fastest pairing: the fast deflate and the `Up` row filter.
    pub(super) fn encode(&self) -> BrpResult<EncodedCapture> {
        let mut bytes = Vec::new();
        PngEncoder::new_with_quality(&mut bytes, CompressionType::Fast, FilterType::Up)
            .write_image(
                self.0.as_raw(),
                self.0.width(),
                self.0.height(),
                ExtendedColorType::Rgb8,
            )
            .map_err(png_encoding_error)?;

        Ok(EncodedCapture {
            bytes,
            dimensions: UVec2::new(self.0.width(), self.0.height()),
        })
    }
}

/// Copies the extent's rows out of the captured bytes. Bevy strips the GPU row padding from
/// screenshot images, so each row is exactly `width * pixel_size` bytes.
fn crop_rows(image: &Image, extent: URect) -> BrpResult<Image> {
    let format = image.texture_descriptor.format;
    let pixel_size = format.pixel_size().map_err(|_| {
        capture_error(format!(
            "Captured image format {format:?} has no fixed pixel size"
        ))
    })?;
    let data = image
        .data
        .as_ref()
        .ok_or_else(|| capture_error("Captured image has no pixel data"))?;
    let row_size = image.width() as usize * pixel_size;
    let columns = extent.min.x as usize * pixel_size..extent.max.x as usize * pixel_size;
    let mut cropped = Vec::with_capacity(columns.len() * extent.height() as usize);
    for row in data
        .chunks_exact(row_size)
        .skip(extent.min.y as usize)
        .take(extent.height() as usize)
    {
        cropped.extend_from_slice(row.get(columns.clone()).unwrap_or_default());
    }
    if cropped.len() != columns.len() * extent.height() as usize {
        return Err(capture_error(
            "Captured image data is shorter than its extent",
        ));
    }

    Ok(Image::new(
        Extent3d {
            width:                 extent.width(),
            height:                extent.height(),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        cropped,
        format,
        image.asset_usage,
    ))
}

pub(super) struct EncodedCapture {
    pub(super) bytes:      Vec<u8>,
    pub(super) dimensions: UVec2,
}

fn capture_error(message: impl Into<String>) -> BrpError {
    BrpError {
        code:    INTERNAL_ERROR,
        message: message.into(),
        data:    None,
    }
}

fn png_encoding_error(error: ImageError) -> BrpError {
    capture_error(format!("Failed to encode captured image as PNG: {error}"))
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::io;
    use std::io::Error as IoError;

    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::TextureFormat;
    use image::GenericImageView;
    use image::ImageFormat;

    use super::*;

    const FIRST_PIXEL: [u8; 4] = [10, 20, 30, 240];
    const SECOND_PIXEL: [u8; 4] = [40, 50, 60, 230];
    const THIRD_PIXEL: [u8; 4] = [70, 80, 90, 220];
    const FOURTH_PIXEL: [u8; 4] = [100, 110, 120, 210];

    fn test_image() -> Image {
        Image::new(
            Extent3d {
                width:                 2,
                height:                2,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            [FIRST_PIXEL, SECOND_PIXEL, THIRD_PIXEL, FOURTH_PIXEL].concat(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD,
        )
    }

    fn capture(image: Image, crop: Option<URect>) -> Result<EncodedCapture, IoError> {
        TargetRgbImage::from_capture(image, crop)
            .and_then(|target_image| target_image.encode())
            .map_err(|error| io::Error::other(error.message))
    }

    fn bgra_test_image() -> Image {
        let swizzled = [FIRST_PIXEL, SECOND_PIXEL, THIRD_PIXEL, FOURTH_PIXEL]
            .map(|[red, green, blue, alpha]| [blue, green, red, alpha]);
        Image::new(
            Extent3d {
                width:                 2,
                height:                2,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            swizzled.concat(),
            TextureFormat::Bgra8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD,
        )
    }

    #[test]
    fn exact_full_and_crop_pngs_preserve_rgb_pixels() -> Result<(), Box<dyn Error>> {
        let full = capture(test_image(), None)?;
        let crop = capture(test_image(), Some(URect::new(1, 0, 2, 2)))?;

        let full_image = image::load_from_memory_with_format(&full.bytes, ImageFormat::Png)?;
        let crop_image = image::load_from_memory_with_format(&crop.bytes, ImageFormat::Png)?;

        assert_eq!(full.dimensions, UVec2::new(2, 2));
        assert_eq!(crop.dimensions, UVec2::new(1, 2));
        assert_eq!(full_image.dimensions(), (2, 2));
        assert_eq!(crop_image.dimensions(), (1, 2));
        assert_eq!(full_image.to_rgb8().get_pixel(0, 0).0, FIRST_PIXEL[..3]);
        assert_eq!(crop_image.to_rgb8().get_pixel(0, 0).0, SECOND_PIXEL[..3]);
        assert_eq!(crop_image.to_rgb8().get_pixel(0, 1).0, FOURTH_PIXEL[..3]);
        Ok(())
    }

    #[test]
    fn bgra_crop_swizzles_only_the_kept_pixels_to_rgb() -> Result<(), Box<dyn Error>> {
        let crop = capture(bgra_test_image(), Some(URect::new(0, 1, 2, 2)))?;
        let crop_image = image::load_from_memory_with_format(&crop.bytes, ImageFormat::Png)?;

        assert_eq!(crop_image.dimensions(), (2, 1));
        assert_eq!(crop_image.to_rgb8().get_pixel(0, 0).0, THIRD_PIXEL[..3]);
        assert_eq!(crop_image.to_rgb8().get_pixel(1, 0).0, FOURTH_PIXEL[..3]);
        Ok(())
    }

    #[test]
    fn hdr_brightness_alpha_is_not_encoded_as_png_alpha() -> Result<(), Box<dyn Error>> {
        let encoded = capture(test_image(), None)?;
        let decoded = image::load_from_memory_with_format(&encoded.bytes, ImageFormat::Png)?;

        assert_eq!(decoded.color(), image::ColorType::Rgb8);
        assert_eq!(decoded.to_rgb8().get_pixel(0, 0).0, FIRST_PIXEL[..3]);
        Ok(())
    }

    #[test]
    fn crop_fails_when_the_captured_extent_is_smaller_than_promised() {
        let result = TargetRgbImage::from_capture(test_image(), Some(URect::new(1, 1, 3, 3)));

        assert!(result.is_err());
    }
}
