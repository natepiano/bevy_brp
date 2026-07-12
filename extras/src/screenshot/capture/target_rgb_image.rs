//! Captured target conversion, cropping, and PNG encoding.

use std::io::Cursor;

use bevy::prelude::*;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
use image::ImageError;
use image::ImageFormat;
use image::RgbImage;

pub(super) struct TargetRgbImage(RgbImage);

impl TryFrom<Image> for TargetRgbImage {
    type Error = BrpError;

    fn try_from(image: Image) -> Result<Self, Self::Error> {
        image
            .try_into_dynamic()
            .map(|dynamic_image| Self(dynamic_image.to_rgb8()))
            .map_err(|error| {
                capture_error(format!("Failed to convert captured image to RGB: {error}"))
            })
    }
}

impl TargetRgbImage {
    pub(super) fn encode(&self, crop: Option<URect>) -> BrpResult<EncodedCapture> {
        let actual_extent =
            URect::from_corners(UVec2::ZERO, UVec2::new(self.0.width(), self.0.height()));
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

        let mut cursor = Cursor::new(Vec::new());
        if capture_extent == actual_extent {
            self.0
                .write_to(&mut cursor, ImageFormat::Png)
                .map_err(png_encoding_error)?;
        } else {
            image::imageops::crop_imm(
                &self.0,
                capture_extent.min.x,
                capture_extent.min.y,
                capture_extent.width(),
                capture_extent.height(),
            )
            .to_image()
            .write_to(&mut cursor, ImageFormat::Png)
            .map_err(png_encoding_error)?;
        }

        Ok(EncodedCapture {
            bytes:      cursor.into_inner(),
            dimensions: capture_extent.size(),
        })
    }
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
    use bevy::render::render_resource::Extent3d;
    use bevy::render::render_resource::TextureDimension;
    use bevy::render::render_resource::TextureFormat;
    use image::GenericImageView;

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

    fn convert(image: Image) -> Result<TargetRgbImage, IoError> {
        TargetRgbImage::try_from(image).map_err(|error| io::Error::other(error.message))
    }

    fn encode(
        target_image: &TargetRgbImage,
        crop: Option<URect>,
    ) -> Result<EncodedCapture, IoError> {
        target_image
            .encode(crop)
            .map_err(|error| io::Error::other(error.message))
    }

    #[test]
    fn exact_full_and_crop_pngs_preserve_rgb_pixels() -> Result<(), Box<dyn Error>> {
        let target_image = convert(test_image())?;
        let full = encode(&target_image, None)?;
        let crop = encode(&target_image, Some(URect::new(1, 0, 2, 2)))?;

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
    fn hdr_brightness_alpha_is_not_encoded_as_png_alpha() -> Result<(), Box<dyn Error>> {
        let target_image = convert(test_image())?;
        let encoded = encode(&target_image, None)?;
        let decoded = image::load_from_memory_with_format(&encoded.bytes, ImageFormat::Png)?;

        assert_eq!(decoded.color(), image::ColorType::Rgb8);
        assert_eq!(decoded.to_rgb8().get_pixel(0, 0).0, FIRST_PIXEL[..3]);
        Ok(())
    }

    #[test]
    fn crop_fails_when_the_captured_extent_is_smaller_than_promised() -> Result<(), IoError> {
        let target_image = convert(test_image())?;

        let result = target_image.encode(Some(URect::new(1, 1, 3, 3)));

        assert!(result.is_err());
        Ok(())
    }
}
