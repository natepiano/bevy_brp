//! Screenshot jobs, worker completion, and temporary-file ownership.

use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;

use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::tasks::IoTaskPool;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
use tempfile::NamedTempFile;
use tempfile::TempPath;

use super::target_rgb_image::EncodedCapture;
use super::target_rgb_image::TargetRgbImage;
use crate::screenshot::CaptureResponseMetadata;

pub(super) type ImageConverter = fn(Image) -> BrpResult<TargetRgbImage>;

pub(super) struct ScreenshotJob {
    pub(super) path:              PathBuf,
    pub(super) crop:              Option<URect>,
    pub(super) response_metadata: CaptureResponseMetadata,
}

pub(super) struct CaptureMetadata {
    pub(super) dimensions:        UVec2,
    pub(super) response_metadata: CaptureResponseMetadata,
}

pub(super) struct OwnedTempCapture {
    pub(super) metadata:  CaptureMetadata,
    pub(super) temp_path: TempPath,
}

pub(super) struct WorkerCompletion {
    pub(super) result: BrpResult<OwnedTempCapture>,
}

pub(super) struct CaptureCompletionChannel {
    pub(super) receiver:  Mutex<Receiver<WorkerCompletion>>,
    pub(super) sender:    Sender<WorkerCompletion>,
    pub(super) converter: ImageConverter,
}

impl Default for CaptureCompletionChannel {
    fn default() -> Self {
        let (sender, receiver) = mpsc::channel();
        Self {
            receiver: Mutex::new(receiver),
            sender,
            converter: TargetRgbImage::try_from,
        }
    }
}

pub(super) fn start_capture_worker(
    image: Image,
    screenshot_job: ScreenshotJob,
    sender: Sender<WorkerCompletion>,
    converter: ImageConverter,
) {
    AsyncComputeTaskPool::get()
        .spawn(async move {
            let prepared_job = prepare_capture_job(image, screenshot_job, converter);
            match prepared_job.encoded_capture {
                Ok(encoded_capture) => {
                    IoTaskPool::get()
                        .spawn(async move {
                            let completion =
                                write_temporary_capture(prepared_job.job, encoded_capture);
                            let _ = sender.send(completion);
                        })
                        .detach();
                },
                Err(error) => {
                    let _ = sender.send(WorkerCompletion { result: Err(error) });
                },
            }
        })
        .detach();
}

struct PreparedJob {
    job:             ScreenshotJob,
    encoded_capture: BrpResult<EncodedCapture>,
}

fn prepare_capture_job(
    image: Image,
    screenshot_job: ScreenshotJob,
    converter: ImageConverter,
) -> PreparedJob {
    match converter(image) {
        Ok(target_image) => PreparedJob {
            encoded_capture: target_image.encode(screenshot_job.crop),
            job:             screenshot_job,
        },
        Err(error) => PreparedJob {
            job:             screenshot_job,
            encoded_capture: Err(error),
        },
    }
}

fn write_temporary_capture(
    job: ScreenshotJob,
    encoded_capture: EncodedCapture,
) -> WorkerCompletion {
    let result = create_temporary_file(&job.path, &encoded_capture.bytes).map(|temp_path| {
        OwnedTempCapture {
            metadata: CaptureMetadata {
                dimensions:        encoded_capture.dimensions,
                response_metadata: job.response_metadata.clone(),
            },
            temp_path,
        }
    });

    WorkerCompletion { result }
}

pub(super) fn create_temporary_file(destination: &Path, bytes: &[u8]) -> BrpResult<TempPath> {
    let parent = destination.parent().ok_or_else(|| {
        capture_error(format!(
            "Screenshot destination {} has no parent directory",
            destination.display()
        ))
    })?;
    std::fs::create_dir_all(parent).map_err(|error| {
        capture_error(format!(
            "Failed to create screenshot directory {}: {error}",
            parent.display()
        ))
    })?;

    let mut named_temp_file = NamedTempFile::new_in(parent).map_err(|error| {
        capture_error(format!(
            "Failed to create temporary screenshot beside {}: {error}",
            destination.display()
        ))
    })?;
    named_temp_file.write_all(bytes).map_err(|error| {
        capture_error(format!(
            "Failed to write temporary screenshot for {}: {error}",
            destination.display()
        ))
    })?;
    named_temp_file.flush().map_err(|error| {
        capture_error(format!(
            "Failed to flush temporary screenshot for {}: {error}",
            destination.display()
        ))
    })?;
    let (file, temp_path) = named_temp_file.into_parts();
    drop(file);
    Ok(temp_path)
}

fn capture_error(message: impl Into<String>) -> BrpError {
    BrpError {
        code:    INTERNAL_ERROR,
        message: message.into(),
        data:    None,
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs;
    use std::io;
    use std::io::Error as IoError;

    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::Extent3d;
    use bevy::render::render_resource::TextureDimension;
    use bevy::render::render_resource::TextureFormat;
    use tempfile::TempDir;

    use super::*;
    use crate::screenshot::CaptureResponseMetadata;

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

    fn job(path: PathBuf, crop: Option<URect>) -> ScreenshotJob {
        ScreenshotJob {
            path,
            crop,
            response_metadata: CaptureResponseMetadata::Full,
        }
    }

    fn encoded(prepared_job: PreparedJob) -> Result<EncodedCapture, IoError> {
        prepared_job
            .encoded_capture
            .map_err(|error| io::Error::other(error.message))
    }

    #[test]
    fn worker_preparation_encodes_the_requested_crop() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let prepared_job = prepare_capture_job(
            test_image(),
            job(
                temp_dir.path().join("crop.png"),
                Some(URect::new(1, 0, 2, 2)),
            ),
            TargetRgbImage::try_from,
        );

        let encoded_capture = encoded(prepared_job)?;
        assert_eq!(encoded_capture.dimensions, UVec2::new(1, 2));
        Ok(())
    }

    #[test]
    fn temporary_path_owns_the_file_until_publication_or_drop() -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        let destination = temp_dir.path().join("capture.png");
        let temp_path = create_temporary_file(&destination, b"owned")
            .map_err(|error| io::Error::other(error.message))?;
        let owned_path = temp_path.to_path_buf();

        assert!(owned_path.exists());
        drop(temp_path);
        assert!(!owned_path.exists());

        let blocking_file = temp_dir.path().join("blocking-file");
        fs::write(&blocking_file, b"not a directory")?;
        let error_destination = blocking_file.join("capture.png");
        assert!(create_temporary_file(&error_destination, b"bytes").is_err());
        Ok(())
    }
}
