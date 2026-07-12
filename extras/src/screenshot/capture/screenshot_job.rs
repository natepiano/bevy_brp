//! Screenshot jobs, worker completion, and temporary-file ownership.

use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Instant;

use bevy::prelude::*;
use bevy::tasks::AsyncComputeTaskPool;
use bevy::tasks::IoTaskPool;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
use tempfile::NamedTempFile;
use tempfile::TempPath;

use super::identity::CaptureIdentity;
use super::identity::PathGeneration;
use super::target_rgb_image::EncodedCapture;
use super::target_rgb_image::TargetRgbImage;

pub(super) type ImageConverter = fn(Image) -> BrpResult<TargetRgbImage>;

pub(super) struct ScreenshotJob {
    pub(super) path:            PathBuf,
    pub(super) crop:            Option<URect>,
    pub(super) identity:        CaptureIdentity,
    pub(super) path_generation: PathGeneration,
    pub(super) deadline:        Instant,
}

pub(super) struct CaptureMetadata {
    pub(super) dimensions: UVec2,
}

pub(super) struct OwnedTempCapture {
    pub(super) identity:        CaptureIdentity,
    pub(super) metadata:        CaptureMetadata,
    pub(super) path_generation: PathGeneration,
    pub(super) temp_path:       TempPath,
}

pub(super) struct WorkerCompletion {
    pub(super) deadline:        Instant,
    pub(super) identity:        CaptureIdentity,
    pub(super) path:            PathBuf,
    pub(super) path_generation: PathGeneration,
    pub(super) result:          BrpResult<OwnedTempCapture>,
}

impl WorkerCompletion {
    fn failed(job: ScreenshotJob, error: BrpError) -> Self {
        Self {
            deadline:        job.deadline,
            identity:        job.identity,
            path:            job.path,
            path_generation: job.path_generation,
            result:          Err(error),
        }
    }
}

#[derive(Resource)]
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
    jobs: Vec<ScreenshotJob>,
    sender: Sender<WorkerCompletion>,
    converter: ImageConverter,
) {
    AsyncComputeTaskPool::get()
        .spawn(async move {
            for prepared_job in prepare_capture_jobs(image, jobs, converter) {
                let completion_sender = sender.clone();
                match prepared_job.encoded_capture {
                    Ok(encoded_capture) => {
                        IoTaskPool::get()
                            .spawn(async move {
                                let completion =
                                    write_temporary_capture(prepared_job.job, encoded_capture);
                                let _ = completion_sender.send(completion);
                            })
                            .detach();
                    },
                    Err(error) => {
                        let _ = completion_sender
                            .send(WorkerCompletion::failed(prepared_job.job, error));
                    },
                }
            }
        })
        .detach();
}

struct PreparedJob {
    job:             ScreenshotJob,
    encoded_capture: BrpResult<EncodedCapture>,
}

fn prepare_capture_jobs(
    image: Image,
    jobs: Vec<ScreenshotJob>,
    converter: ImageConverter,
) -> Vec<PreparedJob> {
    match converter(image) {
        Ok(target_image) => jobs
            .into_iter()
            .map(|job| PreparedJob {
                encoded_capture: target_image.encode(job.crop),
                job,
            })
            .collect(),
        Err(error) => jobs
            .into_iter()
            .map(|job| PreparedJob {
                job,
                encoded_capture: Err(error.clone()),
            })
            .collect(),
    }
}

fn write_temporary_capture(
    job: ScreenshotJob,
    encoded_capture: EncodedCapture,
) -> WorkerCompletion {
    let result = create_temporary_file(&job.path, &encoded_capture.bytes).map(|temp_path| {
        OwnedTempCapture {
            identity: job.identity.clone(),
            metadata: CaptureMetadata {
                dimensions: encoded_capture.dimensions,
            },
            path_generation: job.path_generation,
            temp_path,
        }
    });

    WorkerCompletion {
        deadline: job.deadline,
        identity: job.identity,
        path: job.path,
        path_generation: job.path_generation,
        result,
    }
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
    use std::sync::atomic::AtomicUsize;
    use std::sync::atomic::Ordering;

    use bevy::asset::RenderAssetUsages;
    use bevy::render::render_resource::Extent3d;
    use bevy::render::render_resource::TextureDimension;
    use bevy::render::render_resource::TextureFormat;
    use image::GenericImageView;
    use image::ImageFormat;
    use tempfile::TempDir;

    use super::*;
    use crate::screenshot::capture::RequestFingerprint;

    const FIRST_PIXEL: [u8; 4] = [10, 20, 30, 240];
    const SECOND_PIXEL: [u8; 4] = [40, 50, 60, 230];
    const THIRD_PIXEL: [u8; 4] = [70, 80, 90, 220];
    const FOURTH_PIXEL: [u8; 4] = [100, 110, 120, 210];
    const TEST_PATH_GENERATION: PathGeneration = PathGeneration(1);
    static CONVERSIONS: AtomicUsize = AtomicUsize::new(0);

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

    fn counting_conversion(image: Image) -> BrpResult<TargetRgbImage> {
        CONVERSIONS.fetch_add(1, Ordering::SeqCst);
        TargetRgbImage::try_from(image)
    }

    fn job(path: PathBuf, crop: Option<URect>) -> ScreenshotJob {
        ScreenshotJob {
            identity: CaptureIdentity::Legacy(RequestFingerprint::from(path.clone())),
            path,
            crop,
            path_generation: TEST_PATH_GENERATION,
            deadline: Instant::now(),
        }
    }

    fn encoded(prepared_job: PreparedJob) -> Result<EncodedCapture, IoError> {
        prepared_job
            .encoded_capture
            .map_err(|error| io::Error::other(error.message))
    }

    #[test]
    fn worker_preparation_converts_once_and_fans_out_full_and_crop_jobs()
    -> Result<(), Box<dyn Error>> {
        let temp_dir = TempDir::new()?;
        CONVERSIONS.store(0, Ordering::SeqCst);
        let jobs = vec![
            job(temp_dir.path().join("full.png"), None),
            job(
                temp_dir.path().join("crop.png"),
                Some(URect::new(1, 0, 2, 2)),
            ),
        ];

        let prepared = prepare_capture_jobs(test_image(), jobs, counting_conversion);

        assert_eq!(CONVERSIONS.load(Ordering::SeqCst), 1);
        let mut prepared = prepared.into_iter();
        let full = encoded(
            prepared
                .next()
                .ok_or_else(|| io::Error::other("missing full capture"))?,
        )?;
        let crop = encoded(
            prepared
                .next()
                .ok_or_else(|| io::Error::other("missing crop capture"))?,
        )?;
        assert!(prepared.next().is_none());

        let full_image = image::load_from_memory_with_format(&full.bytes, ImageFormat::Png)?;
        let crop_image = image::load_from_memory_with_format(&crop.bytes, ImageFormat::Png)?;
        assert_eq!(full_image.dimensions(), (2, 2));
        assert_eq!(crop_image.dimensions(), (1, 2));
        assert_eq!(crop_image.to_rgb8().get_pixel(0, 0).0, SECOND_PIXEL[..3]);
        assert_eq!(crop_image.to_rgb8().get_pixel(0, 1).0, FOURTH_PIXEL[..3]);
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
