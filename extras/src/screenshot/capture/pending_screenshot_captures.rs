//! Pending screenshot watcher lifecycle and PNG publication.

use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;

use bevy::camera::NormalizedRenderTarget;
use bevy::prelude::*;
use bevy::render::view::screenshot::Screenshot;
use bevy::render::view::screenshot::ScreenshotCaptured;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INTERNAL_ERROR;
use bevy_remote::error_codes::INVALID_PARAMS;
use serde_json::Value;

use super::identity::CaptureIdentity;
use super::identity::FrameGeneration;
use super::identity::PathGeneration;
use super::identity::RequestFingerprint;
use super::screenshot_job;
use super::screenshot_job::CaptureCompletionChannel;
use super::screenshot_job::OwnedTempCapture;
use super::screenshot_job::ScreenshotJob;
use super::screenshot_job::WorkerCompletion;
use crate::constants::SCREENSHOT_CAPTURE_DEADLINE;
use crate::constants::SCREENSHOT_ENTITY_NAME;
use crate::screenshot;
use crate::screenshot::request::ScreenshotRequest;

#[derive(Resource, Default)]
pub struct PendingScreenshotCaptures {
    captures:         HashMap<CaptureIdentity, CaptureRecord>,
    current_frame:    FrameGeneration,
    path_generations: HashMap<PathBuf, PathGeneration>,
    reservations:     HashMap<PathBuf, DestinationReservation>,
    target_batches:   HashMap<NormalizedRenderTarget, TargetBatch>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CaptureState {
    Pending,
    Encoding,
    ReadyToPublish,
    Publishing,
    Completed,
    Failed,
    TimedOut,
    Abandoned,
}

impl CaptureState {
    const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::TimedOut | Self::Abandoned
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeliveryStatus {
    Awaiting,
    Delivered(FrameGeneration),
}

struct CaptureRecord {
    deadline:        Instant,
    delivery_status: DeliveryStatus,
    error:           Option<BrpError>,
    fingerprint:     RequestFingerprint,
    path:            PathBuf,
    path_generation: PathGeneration,
    result:          Option<Value>,
    seen_frame:      FrameGeneration,
    state:           CaptureState,
}

impl CaptureRecord {
    fn read(&mut self, current_frame: FrameGeneration) -> BrpResult<Option<Value>> {
        if let DeliveryStatus::Delivered(delivery_frame) = self.delivery_status {
            return if delivery_frame == current_frame {
                self.terminal_result()
            } else {
                Ok(None)
            };
        }

        match self.state {
            CaptureState::Completed | CaptureState::Failed | CaptureState::TimedOut => {
                self.delivery_status = DeliveryStatus::Delivered(current_frame);
                self.terminal_result()
            },
            CaptureState::Pending
            | CaptureState::Encoding
            | CaptureState::ReadyToPublish
            | CaptureState::Publishing
            | CaptureState::Abandoned => Ok(None),
        }
    }

    fn terminal_result(&self) -> BrpResult<Option<Value>> {
        match self.state {
            CaptureState::Completed => self
                .result
                .clone()
                .map(Some)
                .ok_or_else(|| invalid_completed_state(&self.path)),
            CaptureState::Failed | CaptureState::TimedOut => Err(self
                .error
                .clone()
                .unwrap_or_else(|| invalid_terminal_state(&self.path))),
            CaptureState::Pending
            | CaptureState::Encoding
            | CaptureState::ReadyToPublish
            | CaptureState::Publishing
            | CaptureState::Abandoned => Ok(None),
        }
    }
}

struct DestinationReservation {
    fingerprint:     RequestFingerprint,
    generation:      PathGeneration,
    owner:           CaptureIdentity,
    watcher_ids:     HashSet<CaptureIdentity>,
    worker_deadline: Instant,
    work:            ReservationWork,
}

impl DestinationReservation {
    fn owns(&self, completion: &WorkerCompletion) -> bool {
        self.generation == completion.path_generation
            && self.owner == completion.identity
            && self.worker_deadline == completion.deadline
    }

    const fn is_acknowledged(&self) -> bool {
        matches!(
            self.work,
            ReservationWork::Published(_)
                | ReservationWork::Failed(_)
                | ReservationWork::Suppressed
        )
    }
}

enum ReservationWork {
    Capturing,
    Encoding,
    Ready {
        capture:     OwnedTempCapture,
        ready_frame: FrameGeneration,
    },
    Publishing,
    Published(Value),
    Failed(BrpError),
    Suppressed,
}

impl ReservationWork {
    const fn owns_incomplete_generation(&self) -> bool {
        matches!(
            self,
            Self::Capturing | Self::Encoding | Self::Ready { .. } | Self::Publishing
        )
    }
}

struct TargetBatch {
    jobs: Vec<ScreenshotJob>,
}

struct PublicationClaim {
    capture:         OwnedTempCapture,
    path:            PathBuf,
    path_generation: PathGeneration,
}

pub(super) struct CaptureDispatch {
    pub(super) response:     Option<Value>,
    pub(super) spawn_target: Option<NormalizedRenderTarget>,
}

impl PendingScreenshotCaptures {
    pub(super) fn read_existing(
        &mut self,
        request: &ScreenshotRequest,
    ) -> Option<BrpResult<CaptureDispatch>> {
        let record = self.captures.get_mut(request.identity())?;
        if &record.fingerprint != request.fingerprint() {
            return Some(Err(token_fingerprint_error()));
        }

        record.seen_frame = self.current_frame;
        Some(
            record
                .read(self.current_frame)
                .map(|response| CaptureDispatch {
                    response,
                    spawn_target: None,
                }),
        )
    }

    pub(super) fn handle(
        &mut self,
        request: ScreenshotRequest,
        target: NormalizedRenderTarget,
    ) -> BrpResult<CaptureDispatch> {
        self.handle_at(request, target, Instant::now())
    }

    fn handle_at(
        &mut self,
        request: ScreenshotRequest,
        target: NormalizedRenderTarget,
        now: Instant,
    ) -> BrpResult<CaptureDispatch> {
        if let Some(dispatch) = self.read_existing(&request) {
            return dispatch;
        }

        let (path, fingerprint, identity) = request.into_parts();

        if let Some(reservation) = self.reservations.get_mut(&path) {
            if now >= reservation.worker_deadline && reservation.work.owns_incomplete_generation() {
                return Err(expired_generation_error(&path));
            }
            if reservation.fingerprint != fingerprint {
                return Err(destination_conflict_error(&path));
            }

            reservation.watcher_ids.insert(identity.clone());
            let (state, result, error) = watcher_state(&reservation.work);
            self.captures.insert(
                identity,
                CaptureRecord {
                    deadline: reservation.worker_deadline,
                    delivery_status: DeliveryStatus::Awaiting,
                    error,
                    fingerprint,
                    path,
                    path_generation: reservation.generation,
                    result,
                    seen_frame: self.current_frame,
                    state,
                },
            );
            return Ok(CaptureDispatch {
                response:     None,
                spawn_target: None,
            });
        }

        let path_generation = self.next_path_generation(&path)?;
        let deadline = now + SCREENSHOT_CAPTURE_DEADLINE;
        let screenshot_job = ScreenshotJob {
            path: path.clone(),
            crop: None,
            identity: identity.clone(),
            path_generation,
            deadline,
        };
        let spawn_target = if let Some(batch) = self.target_batches.get_mut(&target) {
            batch.jobs.push(screenshot_job);
            None
        } else {
            self.target_batches.insert(
                target.clone(),
                TargetBatch {
                    jobs: vec![screenshot_job],
                },
            );
            Some(target)
        };

        self.reservations.insert(
            path.clone(),
            DestinationReservation {
                fingerprint:     fingerprint.clone(),
                generation:      path_generation,
                owner:           identity.clone(),
                watcher_ids:     HashSet::from([identity.clone()]),
                worker_deadline: deadline,
                work:            ReservationWork::Capturing,
            },
        );
        self.captures.insert(
            identity,
            CaptureRecord {
                deadline,
                delivery_status: DeliveryStatus::Awaiting,
                error: None,
                fingerprint,
                path,
                path_generation,
                result: None,
                seen_frame: self.current_frame,
                state: CaptureState::Pending,
            },
        );

        Ok(CaptureDispatch {
            response: None,
            spawn_target,
        })
    }

    fn next_path_generation(&mut self, path: &Path) -> BrpResult<PathGeneration> {
        let generation = self.path_generations.entry(path.to_path_buf()).or_default();
        generation.0 = generation.0.checked_add(1).ok_or_else(|| BrpError {
            code:    INTERNAL_ERROR,
            message: format!(
                "Screenshot path generation overflowed for {}",
                path.display()
            ),
            data:    None,
        })?;
        Ok(*generation)
    }

    fn begin_frame(&mut self, completions: Vec<WorkerCompletion>) {
        self.begin_frame_at(completions, Instant::now());
    }

    fn begin_frame_at(&mut self, completions: Vec<WorkerCompletion>, now: Instant) {
        self.current_frame = self.current_frame.next();
        for completion in completions {
            self.ingest_completion(completion, now);
        }
        self.remove_inactive_reservations();
    }

    fn ingest_completion(&mut self, completion: WorkerCompletion, now: Instant) {
        self.acknowledge_target_job(&completion);
        let Some(reservation) = self.reservations.get(&completion.path) else {
            return;
        };
        if !reservation.owns(&completion)
            || self.path_generations.get(&completion.path) != Some(&completion.path_generation)
            || !matches!(
                reservation.work,
                ReservationWork::Capturing | ReservationWork::Encoding
            )
        {
            return;
        }
        let worker_expired = now >= reservation.worker_deadline;
        let has_active_watcher = self.reservation_has_active_watcher(&completion.path);
        let Some(reservation) = self.reservations.get_mut(&completion.path) else {
            return;
        };

        if worker_expired {
            drop(completion.result);
            let error = timeout_error();
            reservation.work = ReservationWork::Failed(error.clone());
            self.timeout_active_watchers(&completion.path, error);
            return;
        }

        match completion.result {
            Ok(capture) if has_active_watcher => {
                reservation.work = ReservationWork::Ready {
                    capture,
                    ready_frame: self.current_frame,
                };
                self.set_active_watcher_state(&completion.path, CaptureState::ReadyToPublish);
            },
            Ok(capture) => {
                drop(capture);
                reservation.work = ReservationWork::Suppressed;
            },
            Err(error) => {
                reservation.work = ReservationWork::Failed(error.clone());
                self.fail_active_watchers(&completion.path, error);
            },
        }
    }

    fn acknowledge_target_job(&mut self, completion: &WorkerCompletion) {
        for batch in self.target_batches.values_mut() {
            batch.jobs.retain(|job| {
                job.path != completion.path
                    || job.path_generation != completion.path_generation
                    || job.identity != completion.identity
            });
        }
        self.target_batches
            .retain(|_, batch| !batch.jobs.is_empty());
    }

    fn take_target_batch(&mut self, target: &NormalizedRenderTarget) -> Option<Vec<ScreenshotJob>> {
        let batch = self.target_batches.remove(target)?;
        for job in &batch.jobs {
            let Some(reservation) = self.reservations.get_mut(&job.path) else {
                continue;
            };
            if reservation.generation == job.path_generation
                && matches!(reservation.work, ReservationWork::Capturing)
            {
                reservation.work = ReservationWork::Encoding;
                self.set_active_watcher_state(&job.path, CaptureState::Encoding);
            }
        }
        Some(batch.jobs)
    }

    fn advance(&mut self, now: Instant) {
        self.update_watcher_liveness(now);
        let claims = self.claim_publications();
        for claim in claims {
            self.publish(claim);
        }
    }

    fn cleanup(&mut self) {
        self.cleanup_watchers();
        self.remove_inactive_reservations();
    }

    fn update_watcher_liveness(&mut self, now: Instant) {
        for record in self.captures.values_mut() {
            if matches!(record.delivery_status, DeliveryStatus::Delivered(_)) {
                continue;
            }
            if record.seen_frame != self.current_frame {
                record.state = CaptureState::Abandoned;
                record.error = None;
                record.result = None;
            } else if !record.state.is_terminal() && now >= record.deadline {
                record.state = CaptureState::TimedOut;
                record.error = Some(timeout_error());
            }
        }
    }

    fn claim_publications(&mut self) -> Vec<PublicationClaim> {
        let paths = self
            .reservations
            .iter()
            .filter_map(|(path, reservation)| match reservation.work {
                ReservationWork::Ready { ready_frame, .. }
                    if ready_frame < self.current_frame
                        && self.reservation_has_publishable_watcher(path) =>
                {
                    Some(path.clone())
                },
                _ => None,
            })
            .collect::<Vec<_>>();

        let mut claims = Vec::with_capacity(paths.len());
        for path in paths {
            let Some(reservation) = self.reservations.get_mut(&path) else {
                continue;
            };
            let ReservationWork::Ready { capture, .. } =
                std::mem::replace(&mut reservation.work, ReservationWork::Publishing)
            else {
                continue;
            };
            let path_generation = reservation.generation;
            claims.push(PublicationClaim {
                capture,
                path: path.clone(),
                path_generation,
            });
            self.set_publishable_watcher_state(&path, CaptureState::Publishing);
        }

        let suppressed = self
            .reservations
            .iter()
            .filter_map(|(path, reservation)| {
                matches!(reservation.work, ReservationWork::Ready { .. })
                    .then_some(path)
                    .filter(|path| !self.reservation_has_active_watcher(path))
                    .cloned()
            })
            .collect::<Vec<_>>();
        for path in suppressed {
            if let Some(reservation) = self.reservations.get_mut(&path) {
                reservation.work = ReservationWork::Suppressed;
            }
        }

        claims
    }

    fn publish(&mut self, claim: PublicationClaim) {
        if !self.owns_claim(&claim) {
            return;
        }

        let result = claim
            .capture
            .temp_path
            .persist(&claim.path)
            .map_err(|error| {
                let message = format!(
                    "Failed to publish screenshot to {}: {}",
                    claim.path.display(),
                    error.error
                );
                drop(error.path);
                capture_error(message)
            });

        let Some(reservation) = self.reservations.get_mut(&claim.path) else {
            return;
        };
        if reservation.generation != claim.path_generation
            || !matches!(reservation.work, ReservationWork::Publishing)
        {
            return;
        }

        match result {
            Ok(()) => {
                let response = screenshot::completed_response(&claim.path);
                reservation.work = ReservationWork::Published(response.clone());
                self.complete_publishing_watchers(&claim.path, response);
            },
            Err(error) => {
                reservation.work = ReservationWork::Failed(error.clone());
                self.fail_publishing_watchers(&claim.path, error);
            },
        }
    }

    fn owns_claim(&self, claim: &PublicationClaim) -> bool {
        self.path_generations.get(&claim.path) == Some(&claim.path_generation)
            && self
                .reservations
                .get(&claim.path)
                .is_some_and(|reservation| {
                    reservation.generation == claim.path_generation
                        && reservation.owner == claim.capture.identity
                        && reservation.generation == claim.capture.path_generation
                        && claim.capture.metadata.dimensions.cmpgt(UVec2::ZERO).all()
                        && matches!(reservation.work, ReservationWork::Publishing)
                })
    }

    fn cleanup_watchers(&mut self) {
        let current_frame = self.current_frame;
        let removed = self
            .captures
            .extract_if(|_, record| {
                record.state == CaptureState::Abandoned
                    || matches!(
                        record.delivery_status,
                        DeliveryStatus::Delivered(delivery_frame)
                            if delivery_frame < current_frame
                                && record.seen_frame != current_frame
                    )
            })
            .map(|(identity, record)| (identity, record.path, record.path_generation))
            .collect::<Vec<_>>();

        for (identity, path, path_generation) in removed {
            if let Some(reservation) = self.reservations.get_mut(&path)
                && reservation.generation == path_generation
            {
                reservation.watcher_ids.remove(&identity);
            }
        }
    }

    fn remove_inactive_reservations(&mut self) {
        self.reservations.retain(|_, reservation| {
            !reservation.watcher_ids.is_empty() || !reservation.is_acknowledged()
        });
    }

    fn reservation_has_active_watcher(&self, path: &Path) -> bool {
        self.reservations.get(path).is_some_and(|reservation| {
            reservation.watcher_ids.iter().any(|identity| {
                self.captures
                    .get(identity)
                    .is_some_and(|record| !record.state.is_terminal())
            })
        })
    }

    fn reservation_has_publishable_watcher(&self, path: &Path) -> bool {
        self.reservations.get(path).is_some_and(|reservation| {
            reservation.watcher_ids.iter().any(|identity| {
                self.captures.get(identity).is_some_and(|record| {
                    record.seen_frame == self.current_frame
                        && record.state == CaptureState::ReadyToPublish
                })
            })
        })
    }

    fn set_active_watcher_state(&mut self, path: &Path, state: CaptureState) {
        let watcher_ids = self.watcher_ids(path);
        for identity in watcher_ids {
            if let Some(record) = self.captures.get_mut(&identity)
                && !record.state.is_terminal()
            {
                record.state = state;
            }
        }
    }

    fn set_publishable_watcher_state(&mut self, path: &Path, state: CaptureState) {
        let watcher_ids = self.watcher_ids(path);
        for identity in watcher_ids {
            if let Some(record) = self.captures.get_mut(&identity)
                && record.seen_frame == self.current_frame
                && record.state == CaptureState::ReadyToPublish
            {
                record.state = state;
            }
        }
    }

    fn complete_publishing_watchers(&mut self, path: &Path, response: Value) {
        let watcher_ids = self.watcher_ids(path);
        for identity in watcher_ids {
            if let Some(record) = self.captures.get_mut(&identity)
                && record.state == CaptureState::Publishing
            {
                record.state = CaptureState::Completed;
                record.result = Some(response.clone());
            }
        }
    }

    fn fail_active_watchers(&mut self, path: &Path, error: BrpError) {
        let watcher_ids = self.watcher_ids(path);
        for identity in watcher_ids {
            if let Some(record) = self.captures.get_mut(&identity)
                && !record.state.is_terminal()
            {
                record.state = CaptureState::Failed;
                record.error = Some(error.clone());
            }
        }
    }

    fn timeout_active_watchers(&mut self, path: &Path, error: BrpError) {
        let watcher_ids = self.watcher_ids(path);
        for identity in watcher_ids {
            if let Some(record) = self.captures.get_mut(&identity)
                && !record.state.is_terminal()
            {
                record.state = CaptureState::TimedOut;
                record.error = Some(error.clone());
            }
        }
    }

    fn fail_publishing_watchers(&mut self, path: &Path, error: BrpError) {
        let watcher_ids = self.watcher_ids(path);
        for identity in watcher_ids {
            if let Some(record) = self.captures.get_mut(&identity)
                && record.state == CaptureState::Publishing
            {
                record.state = CaptureState::Failed;
                record.error = Some(error.clone());
            }
        }
    }

    fn watcher_ids(&self, path: &Path) -> Vec<CaptureIdentity> {
        self.reservations
            .get(path)
            .map(|reservation| reservation.watcher_ids.iter().cloned().collect())
            .unwrap_or_default()
    }
}

pub(super) fn spawn_primary_window_batch(world: &mut World, target: NormalizedRenderTarget) {
    let observer_target = target;
    world
        .spawn((
            Screenshot::primary_window(),
            Name::new(SCREENSHOT_ENTITY_NAME),
        ))
        .observe(
            move |screenshot_captured: On<ScreenshotCaptured>,
                  mut pending: ResMut<PendingScreenshotCaptures>,
                  channel: Res<CaptureCompletionChannel>| {
                let Some(jobs) = pending.take_target_batch(&observer_target) else {
                    return;
                };
                screenshot_job::start_capture_worker(
                    screenshot_captured.event().image.clone(),
                    jobs,
                    channel.sender.clone(),
                    channel.converter,
                );
            },
        );
}

pub(super) fn ingest_capture_completions(
    mut pending: ResMut<PendingScreenshotCaptures>,
    channel: Res<CaptureCompletionChannel>,
) {
    let Ok(receiver) = channel.receiver.lock() else {
        error!("Screenshot completion channel mutex is poisoned");
        pending.begin_frame(Vec::new());
        return;
    };
    let completions = receiver.try_iter().collect();
    drop(receiver);
    pending.begin_frame(completions);
}

pub(super) fn advance_capture_lifecycle(mut pending: ResMut<PendingScreenshotCaptures>) {
    pending.advance(Instant::now());
}

pub(super) fn cleanup_capture_lifecycle(mut pending: ResMut<PendingScreenshotCaptures>) {
    pending.cleanup();
}

fn watcher_state(work: &ReservationWork) -> (CaptureState, Option<Value>, Option<BrpError>) {
    match work {
        ReservationWork::Capturing => (CaptureState::Pending, None, None),
        ReservationWork::Encoding => (CaptureState::Encoding, None, None),
        ReservationWork::Ready { .. } => (CaptureState::ReadyToPublish, None, None),
        ReservationWork::Publishing => (CaptureState::Publishing, None, None),
        ReservationWork::Published(result) => (CaptureState::Completed, Some(result.clone()), None),
        ReservationWork::Failed(error) => (CaptureState::Failed, None, Some(error.clone())),
        ReservationWork::Suppressed => (
            CaptureState::Failed,
            None,
            Some(capture_error(
                "Screenshot capture no longer has a live caller",
            )),
        ),
    }
}

fn capture_error(message: impl Into<String>) -> BrpError {
    BrpError {
        code:    INTERNAL_ERROR,
        message: message.into(),
        data:    None,
    }
}

fn destination_conflict_error(path: &Path) -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: format!(
            "Screenshot destination {} is active for a different request",
            path.display()
        ),
        data:    None,
    }
}

fn expired_generation_error(path: &Path) -> BrpError {
    capture_error(format!(
        "Screenshot generation for {} exceeded its server deadline and is awaiting worker acknowledgement or cleanup; retry after the current request finishes",
        path.display()
    ))
}

fn invalid_completed_state(path: &Path) -> BrpError {
    capture_error(format!(
        "Completed screenshot capture for {} has no result",
        path.display()
    ))
}

fn invalid_terminal_state(path: &Path) -> BrpError {
    capture_error(format!(
        "Failed screenshot capture for {} has no error",
        path.display()
    ))
}

fn timeout_error() -> BrpError {
    capture_error(format!(
        "Screenshot capture exceeded the {}-second server deadline",
        SCREENSHOT_CAPTURE_DEADLINE.as_secs()
    ))
}

fn token_fingerprint_error() -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: "capture_id is already active for a different screenshot request".to_string(),
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
    use std::time::Duration;

    use async_channel::Receiver as AsyncReceiver;
    use async_channel::TryRecvError;
    use async_channel::bounded;
    use bevy::MinimalPlugins;
    use bevy::asset::RenderAssetUsages;
    use bevy::camera::RenderTarget;
    use bevy::ecs::entity::Entity;
    use bevy::render::render_resource::Extent3d;
    use bevy::render::render_resource::TextureDimension;
    use bevy::render::render_resource::TextureFormat;
    use bevy::window::PrimaryWindow;
    use bevy::window::WindowRef;
    use bevy_remote::BrpMessage;
    use bevy_remote::BrpSender;
    use bevy_remote::RemoteMethodSystemId;
    use bevy_remote::RemoteMethods;
    use bevy_remote::RemotePlugin;
    use serde_json::Value;
    use serde_json::json;
    use tempfile::TempDir;

    use super::*;
    use crate::constants::METHOD_SCREENSHOT;
    use crate::screenshot;
    use crate::screenshot::ScreenshotPlugin;
    use crate::screenshot::capture::screenshot_job;
    use crate::screenshot::capture::screenshot_job::CaptureMetadata;
    use crate::screenshot::capture::target_rgb_image::TargetRgbImage;

    type TestResult = Result<(), Box<dyn Error>>;

    const FIRST_PIXEL: [u8; 4] = [10, 20, 30, 240];
    const SECOND_PIXEL: [u8; 4] = [40, 50, 60, 230];
    const THIRD_PIXEL: [u8; 4] = [70, 80, 90, 220];
    const FOURTH_PIXEL: [u8; 4] = [100, 110, 120, 210];
    const TEST_ENTITY_INDEX: u32 = 1;
    const WORKER_TEST_TIMEOUT: Duration = Duration::from_secs(5);
    static CONVERSIONS: AtomicUsize = AtomicUsize::new(0);

    fn brp<T>(result: BrpResult<T>) -> Result<T, IoError> {
        result.map_err(|error| io::Error::other(error.message))
    }

    fn request(path: &Path, capture_id: Option<&str>) -> Result<ScreenshotRequest, IoError> {
        let mut params = json!({ "path": path });
        if let Some(capture_id) = capture_id {
            params["capture_id"] = json!(capture_id);
        }
        brp(ScreenshotRequest::from_params(Some(params)))
    }

    fn target() -> Result<NormalizedRenderTarget, IoError> {
        let entity = Entity::from_raw_u32(TEST_ENTITY_INDEX)
            .ok_or_else(|| io::Error::other("test entity index was invalid"))?;
        RenderTarget::Window(WindowRef::Primary)
            .normalize(Some(entity))
            .ok_or_else(|| io::Error::other("primary target did not normalize"))
    }

    fn begin_frame(pending: &mut PendingScreenshotCaptures) { pending.begin_frame(Vec::new()); }

    fn finish_frame(pending: &mut PendingScreenshotCaptures) {
        pending.advance(Instant::now());
        pending.cleanup();
    }

    fn finish_frame_at(pending: &mut PendingScreenshotCaptures, now: Instant) {
        pending.advance(now);
        pending.cleanup();
    }

    fn handle(
        pending: &mut PendingScreenshotCaptures,
        path: &Path,
        capture_id: Option<&str>,
    ) -> Result<CaptureDispatch, IoError> {
        brp(pending.handle(request(path, capture_id)?, target()?))
    }

    fn handle_at(
        pending: &mut PendingScreenshotCaptures,
        path: &Path,
        capture_id: Option<&str>,
        now: Instant,
    ) -> Result<CaptureDispatch, IoError> {
        brp(pending.handle_at(request(path, capture_id)?, target()?, now))
    }

    fn identity(path: &Path, capture_id: Option<&str>) -> Result<CaptureIdentity, IoError> {
        Ok(request(path, capture_id)?.identity().clone())
    }

    fn ready_completion(
        pending: &PendingScreenshotCaptures,
        path: &Path,
        bytes: &[u8],
    ) -> Result<WorkerCompletion, IoError> {
        let reservation = pending
            .reservations
            .get(path)
            .ok_or_else(|| io::Error::other("missing destination reservation"))?;
        let temp_path = brp(screenshot_job::create_temporary_file(path, bytes))?;
        Ok(WorkerCompletion {
            deadline:        reservation.worker_deadline,
            identity:        reservation.owner.clone(),
            path:            path.to_path_buf(),
            path_generation: reservation.generation,
            result:          Ok(OwnedTempCapture {
                identity: reservation.owner.clone(),
                metadata: CaptureMetadata {
                    dimensions: UVec2::ONE,
                },
                path_generation: reservation.generation,
                temp_path,
            }),
        })
    }

    fn progress_to_publication(
        pending: &mut PendingScreenshotCaptures,
        path: &Path,
        capture_id: Option<&str>,
        bytes: &[u8],
    ) -> Result<(), IoError> {
        let completion = ready_completion(pending, path, bytes)?;
        pending.begin_frame(vec![completion]);
        handle(pending, path, capture_id)?;
        finish_frame(pending);
        assert!(!path.exists());

        begin_frame(pending);
        handle(pending, path, capture_id)?;
        finish_frame(pending);
        Ok(())
    }

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

    fn remote_app() -> App {
        let mut app = App::new();
        app.add_plugins((RemotePlugin::default(), ScreenshotPlugin));
        let system_id = app.world_mut().register_system(screenshot::handler);
        app.world_mut()
            .resource_mut::<RemoteMethods>()
            .insert(METHOD_SCREENSHOT, RemoteMethodSystemId::Watching(system_id));
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        app
    }

    fn send_remote_request(
        app: &App,
        path: &Path,
        capture_id: &str,
    ) -> Result<AsyncReceiver<BrpResult<Value>>, IoError> {
        let (response_sender, response_receiver) = bounded(1);
        app.world()
            .resource::<BrpSender>()
            .force_send(BrpMessage {
                method: METHOD_SCREENSHOT.to_string(),
                params: Some(json!({
                    "capture_id": capture_id,
                    "path": path,
                })),
                sender: response_sender,
            })
            .map_err(|error| io::Error::other(error.to_string()))?;
        Ok(response_receiver)
    }

    fn receive_terminal(
        response_receiver: &AsyncReceiver<BrpResult<Value>>,
    ) -> Result<Value, IoError> {
        let response = response_receiver
            .try_recv()
            .map_err(|error| io::Error::other(error.to_string()))?;
        brp(response)
    }

    #[test]
    fn repeated_tokens_are_idempotent_and_distinct_tokens_are_isolated() -> TestResult {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("shared.png");
        let mut pending = PendingScreenshotCaptures::default();
        begin_frame(&mut pending);

        let first = handle(&mut pending, &path, Some("first"))?;
        let repeated = handle(&mut pending, &path, Some("first"))?;
        let second = handle(&mut pending, &path, Some("second"))?;

        assert!(first.spawn_target.is_some());
        assert!(repeated.spawn_target.is_none());
        assert!(second.spawn_target.is_none());
        assert_eq!(pending.captures.len(), 2);
        assert_eq!(pending.reservations.len(), 1);
        let jobs = pending
            .target_batches
            .get(&target()?)
            .ok_or_else(|| io::Error::other("missing target batch"))?;
        assert_eq!(jobs.jobs.len(), 1);
        Ok(())
    }

    #[test]
    fn shared_reservation_keeps_distinct_token_delivery_independent() -> TestResult {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("shared-result.png");
        let mut pending = PendingScreenshotCaptures::default();
        begin_frame(&mut pending);
        handle(&mut pending, &path, Some("first"))?;
        handle(&mut pending, &path, Some("second"))?;
        finish_frame(&mut pending);

        let completion = ready_completion(&pending, &path, b"complete")?;
        pending.begin_frame(vec![completion]);
        handle(&mut pending, &path, Some("first"))?;
        handle(&mut pending, &path, Some("second"))?;
        finish_frame(&mut pending);
        begin_frame(&mut pending);
        handle(&mut pending, &path, Some("first"))?;
        handle(&mut pending, &path, Some("second"))?;
        finish_frame(&mut pending);

        begin_frame(&mut pending);
        let first = handle(&mut pending, &path, Some("first"))?;
        let second_identity = identity(&path, Some("second"))?;
        assert_eq!(
            pending
                .captures
                .get(&second_identity)
                .ok_or_else(|| io::Error::other("missing second capture"))?
                .delivery_status,
            DeliveryStatus::Awaiting
        );
        let second = handle(&mut pending, &path, Some("second"))?;
        assert_eq!(first.response, second.response);
        Ok(())
    }

    #[test]
    fn delivery_frame_shares_terminal_result_then_tombstones_until_unseen() -> TestResult {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("terminal.png");
        let mut pending = PendingScreenshotCaptures::default();
        begin_frame(&mut pending);
        handle(&mut pending, &path, Some("terminal"))?;
        finish_frame(&mut pending);
        progress_to_publication(&mut pending, &path, Some("terminal"), b"complete")?;

        begin_frame(&mut pending);
        let first = handle(&mut pending, &path, Some("terminal"))?;
        let concurrent = handle(&mut pending, &path, Some("terminal"))?;
        assert!(first.response.is_some());
        assert_eq!(first.response, concurrent.response);
        finish_frame(&mut pending);
        assert_eq!(pending.captures.len(), 1);

        begin_frame(&mut pending);
        let tombstone = handle(&mut pending, &path, Some("terminal"))?;
        assert!(tombstone.response.is_none());
        assert!(tombstone.spawn_target.is_none());
        finish_frame(&mut pending);
        assert_eq!(pending.captures.len(), 1);

        begin_frame(&mut pending);
        finish_frame(&mut pending);
        assert!(pending.captures.is_empty());
        assert!(pending.reservations.is_empty());
        Ok(())
    }

    #[test]
    fn legacy_tombstone_cleanup_allows_a_later_generation() -> TestResult {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("legacy.png");
        let mut pending = PendingScreenshotCaptures::default();
        begin_frame(&mut pending);
        handle(&mut pending, &path, None)?;
        handle(&mut pending, &path, None)?;
        let first_generation = pending
            .reservations
            .get(&path)
            .ok_or_else(|| io::Error::other("missing first reservation"))?
            .generation;
        finish_frame(&mut pending);
        progress_to_publication(&mut pending, &path, None, b"first")?;

        begin_frame(&mut pending);
        assert!(handle(&mut pending, &path, None)?.response.is_some());
        finish_frame(&mut pending);
        begin_frame(&mut pending);
        assert!(handle(&mut pending, &path, None)?.response.is_none());
        finish_frame(&mut pending);
        begin_frame(&mut pending);
        finish_frame(&mut pending);

        begin_frame(&mut pending);
        let fresh = handle(&mut pending, &path, None)?;
        let second_generation = pending
            .reservations
            .get(&path)
            .ok_or_else(|| io::Error::other("missing second reservation"))?
            .generation;
        assert!(fresh.spawn_target.is_some());
        assert!(second_generation.0 > first_generation.0);
        Ok(())
    }

    #[test]
    fn token_reuse_with_a_different_fingerprint_is_invalid() -> TestResult {
        let temp_dir = TempDir::new()?;
        let first = temp_dir.path().join("first.png");
        let second = temp_dir.path().join("second.png");
        let mut pending = PendingScreenshotCaptures::default();
        begin_frame(&mut pending);

        handle(&mut pending, &first, Some("token"))?;
        let error = pending.handle(request(&second, Some("token"))?, target()?);

        assert!(matches!(error, Err(error) if error.code == INVALID_PARAMS));
        Ok(())
    }

    #[test]
    fn active_destination_with_a_different_fingerprint_is_rejected() -> TestResult {
        let temp_dir = TempDir::new()?;
        let destination = temp_dir.path().join("destination.png");
        let other = temp_dir.path().join("other.png");
        let mut pending = PendingScreenshotCaptures::default();
        begin_frame(&mut pending);
        handle(&mut pending, &destination, Some("first"))?;

        let (_, fingerprint, _) = request(&other, Some("other"))?.into_parts();
        let conflicting_request =
            request(&destination, Some("second"))?.with_fingerprint(fingerprint);
        let error = pending.handle(conflicting_request, target()?);

        assert!(matches!(error, Err(error) if error.code == INVALID_PARAMS));
        assert_eq!(pending.captures.len(), 1);
        assert_eq!(pending.reservations.len(), 1);
        Ok(())
    }

    #[test]
    fn unseen_ready_capture_is_abandoned_before_publication() -> TestResult {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("disconnected.png");
        let mut pending = PendingScreenshotCaptures::default();
        begin_frame(&mut pending);
        handle(&mut pending, &path, Some("disconnect"))?;
        finish_frame(&mut pending);

        let completion = ready_completion(&pending, &path, b"never published")?;
        pending.begin_frame(vec![completion]);
        finish_frame(&mut pending);

        assert!(!path.exists());
        assert!(pending.captures.is_empty());
        assert!(pending.reservations.is_empty());
        Ok(())
    }

    #[test]
    fn existing_requests_remain_readable_without_a_primary_window() -> TestResult {
        let temp_dir = TempDir::new()?;
        let pending_path = temp_dir.path().join("pending.png");
        let completed_path = temp_dir.path().join("completed.png");
        let mut app = App::new();
        app.add_plugins((RemotePlugin::default(), ScreenshotPlugin));
        let primary_window = app
            .world_mut()
            .spawn((Window::default(), PrimaryWindow))
            .id();
        app.update();
        let handler_id = app.world_mut().register_system(screenshot::handler);

        let initial = app
            .world_mut()
            .run_system_with(
                handler_id,
                Some(json!({ "capture_id": "pending", "path": pending_path })),
            )
            .map_err(|error| io::Error::other(error.to_string()))?;
        assert!(brp(initial)?.is_none());
        app.world_mut().despawn(primary_window);

        let pending_read = app
            .world_mut()
            .run_system_with(
                handler_id,
                Some(json!({ "capture_id": "pending", "path": pending_path })),
            )
            .map_err(|error| io::Error::other(error.to_string()))?;
        assert!(brp(pending_read)?.is_none());

        {
            let mut captures = app.world_mut().resource_mut::<PendingScreenshotCaptures>();
            begin_frame(&mut captures);
            handle(&mut captures, &completed_path, Some("completed"))?;
            finish_frame(&mut captures);
            progress_to_publication(
                &mut captures,
                &completed_path,
                Some("completed"),
                b"complete",
            )?;
        }
        let completed_read = app
            .world_mut()
            .run_system_with(
                handler_id,
                Some(json!({
                    "capture_id": "completed",
                    "path": completed_path,
                })),
            )
            .map_err(|error| io::Error::other(error.to_string()))?;
        assert!(brp(completed_read)?.is_some());

        let new_request = app
            .world_mut()
            .run_system_with(
                handler_id,
                Some(json!({ "capture_id": "new", "path": temp_dir.path().join("new.png") })),
            )
            .map_err(|error| io::Error::other(error.to_string()))?;
        assert!(matches!(new_request, Err(error) if error.message.contains("primary window")));
        Ok(())
    }

    #[test]
    fn handler_spawns_one_screenshot_entity_and_observer_takes_batched_jobs() -> TestResult {
        let temp_dir = TempDir::new()?;
        let first_path = temp_dir.path().join("first.png");
        let second_path = temp_dir.path().join("second.png");
        CONVERSIONS.store(0, Ordering::SeqCst);

        let mut app = App::new();
        app.add_plugins((MinimalPlugins, RemotePlugin::default(), ScreenshotPlugin));
        app.world_mut().spawn((Window::default(), PrimaryWindow));
        app.update();
        app.world_mut()
            .resource_mut::<CaptureCompletionChannel>()
            .converter = counting_conversion;
        let handler_id = app.world_mut().register_system(screenshot::handler);

        let first = app
            .world_mut()
            .run_system_with(
                handler_id,
                Some(json!({ "capture_id": "first", "path": first_path })),
            )
            .map_err(|error| io::Error::other(error.to_string()))?;
        brp(first)?;
        let second = app
            .world_mut()
            .run_system_with(
                handler_id,
                Some(json!({ "capture_id": "second", "path": second_path })),
            )
            .map_err(|error| io::Error::other(error.to_string()))?;
        brp(second)?;

        let mut screenshots = app.world_mut().query_filtered::<Entity, With<Screenshot>>();
        let mut screenshot_entities = screenshots.iter(app.world());
        let screenshot_entity = screenshot_entities
            .next()
            .ok_or_else(|| io::Error::other("missing screenshot entity"))?;
        assert!(screenshot_entities.next().is_none());
        app.world_mut()
            .entity_mut(screenshot_entity)
            .trigger(|entity| ScreenshotCaptured {
                entity,
                image: test_image(),
            });

        let channel = app.world().resource::<CaptureCompletionChannel>();
        let receiver = channel
            .receiver
            .lock()
            .map_err(|_| io::Error::other("completion channel mutex poisoned"))?;
        let first_completion = receiver.recv_timeout(WORKER_TEST_TIMEOUT)?;
        let second_completion = receiver.recv_timeout(WORKER_TEST_TIMEOUT)?;
        drop(receiver);
        assert!(first_completion.result.is_ok());
        assert!(second_completion.result.is_ok());
        assert_eq!(CONVERSIONS.load(Ordering::SeqCst), 1);
        assert!(
            app.world()
                .resource::<PendingScreenshotCaptures>()
                .target_batches
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn atomic_publication_replaces_existing_and_creates_absent_destinations() -> TestResult {
        let temp_dir = TempDir::new()?;
        let existing = temp_dir.path().join("replace.png");
        fs::write(&existing, b"sentinel")?;
        let replacement = brp(screenshot_job::create_temporary_file(
            &existing,
            b"complete png",
        ))?;
        replacement.persist(&existing)?;
        assert_eq!(fs::read(&existing)?, b"complete png");

        let absent = temp_dir.path().join("new.png");
        let created = brp(screenshot_job::create_temporary_file(
            &absent,
            b"complete png",
        ))?;
        created.persist(&absent)?;
        assert_eq!(fs::read(&absent)?, b"complete png");
        Ok(())
    }

    #[test]
    fn persist_failure_preserves_existing_destination() -> TestResult {
        let temp_dir = TempDir::new()?;
        let destination = temp_dir.path().join("preserved.png");
        fs::write(&destination, b"sentinel")?;
        let temp_path = brp(screenshot_job::create_temporary_file(
            &destination,
            b"replacement",
        ))?;
        let temporary_file_path: &Path = temp_path.as_ref();
        fs::remove_file(temporary_file_path)?;

        assert!(temp_path.persist(&destination).is_err());
        assert_eq!(fs::read(&destination)?, b"sentinel");
        Ok(())
    }

    #[test]
    fn new_token_cannot_join_an_expired_worker_generation() -> TestResult {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("expired-generation.png");
        let mut pending = PendingScreenshotCaptures::default();
        let started = Instant::now();
        pending.begin_frame_at(Vec::new(), started);
        handle_at(&mut pending, &path, Some("owner"), started)?;
        let reservation = pending
            .reservations
            .get(&path)
            .ok_or_else(|| io::Error::other("missing destination reservation"))?;
        let generation = reservation.generation;
        let worker_deadline = reservation.worker_deadline;

        let rejected = pending.handle_at(
            request(&path, Some("late-watcher"))?,
            target()?,
            worker_deadline,
        );

        assert!(matches!(
            rejected,
            Err(error) if error.message.contains("awaiting worker acknowledgement or cleanup")
        ));
        assert_eq!(pending.captures.len(), 1);
        assert!(
            !pending
                .captures
                .contains_key(&identity(&path, Some("late-watcher"))?)
        );
        let reservation = pending
            .reservations
            .get(&path)
            .ok_or_else(|| io::Error::other("missing destination reservation"))?;
        assert_eq!(reservation.generation, generation);
        assert_eq!(reservation.worker_deadline, worker_deadline);
        assert_eq!(reservation.watcher_ids.len(), 1);
        assert!(matches!(reservation.work, ReservationWork::Capturing));
        assert_eq!(pending.path_generations.get(&path), Some(&generation));
        assert_eq!(
            pending
                .target_batches
                .get(&target()?)
                .ok_or_else(|| io::Error::other("missing target batch"))?
                .jobs
                .len(),
            1
        );
        Ok(())
    }

    #[test]
    fn late_successful_completion_times_out_all_generation_watchers() -> TestResult {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("late-completion.png");
        let mut pending = PendingScreenshotCaptures::default();
        let started = Instant::now();
        pending.begin_frame_at(Vec::new(), started);
        handle_at(&mut pending, &path, Some("owner"), started)?;
        let worker_deadline = pending
            .reservations
            .get(&path)
            .ok_or_else(|| io::Error::other("missing destination reservation"))?
            .worker_deadline;
        handle_at(
            &mut pending,
            &path,
            Some("joined-before-expiration"),
            worker_deadline
                .checked_sub(Duration::from_millis(1))
                .ok_or_else(|| io::Error::other("worker deadline could not move earlier"))?,
        )?;
        assert!(
            pending
                .captures
                .values()
                .all(|record| record.deadline == worker_deadline)
        );

        let completion = ready_completion(&pending, &path, b"late")?;
        let temporary_path = completion
            .result
            .as_ref()
            .map_err(|error| io::Error::other(error.message.clone()))?
            .temp_path
            .to_path_buf();
        pending.begin_frame_at(vec![completion], worker_deadline);

        assert!(!temporary_path.exists());
        assert!(matches!(
            pending
                .reservations
                .get(&path)
                .ok_or_else(|| io::Error::other("missing acknowledged reservation"))?
                .work,
            ReservationWork::Failed(_)
        ));
        assert!(
            pending
                .captures
                .values()
                .all(|record| record.state == CaptureState::TimedOut)
        );
        let owner = pending.handle_at(request(&path, Some("owner"))?, target()?, worker_deadline);
        let joined = pending.handle_at(
            request(&path, Some("joined-before-expiration"))?,
            target()?,
            worker_deadline,
        );
        assert!(matches!(owner, Err(error) if error.message.contains("deadline")));
        assert!(matches!(joined, Err(error) if error.message.contains("deadline")));
        finish_frame_at(&mut pending, worker_deadline);

        assert!(!path.exists());
        assert_eq!(pending.reservations.len(), 1);
        Ok(())
    }

    #[test]
    fn acknowledged_expiration_cleanup_allows_a_greater_generation() -> TestResult {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("expired-then-fresh.png");
        let mut pending = PendingScreenshotCaptures::default();
        let started = Instant::now();
        pending.begin_frame_at(Vec::new(), started);
        handle_at(&mut pending, &path, Some("expired"), started)?;
        let reservation = pending
            .reservations
            .get(&path)
            .ok_or_else(|| io::Error::other("missing destination reservation"))?;
        let first_generation = reservation.generation;
        let worker_deadline = reservation.worker_deadline;
        let completion = ready_completion(&pending, &path, b"late")?;

        pending.begin_frame_at(vec![completion], worker_deadline);
        assert!(
            pending
                .handle_at(request(&path, Some("expired"))?, target()?, worker_deadline)
                .is_err()
        );
        finish_frame_at(&mut pending, worker_deadline);
        pending.begin_frame_at(Vec::new(), worker_deadline);
        finish_frame_at(&mut pending, worker_deadline);
        assert!(pending.captures.is_empty());
        assert!(pending.reservations.is_empty());

        pending.begin_frame_at(Vec::new(), worker_deadline);
        let fresh = handle_at(&mut pending, &path, Some("fresh"), worker_deadline)?;
        let second_generation = pending
            .reservations
            .get(&path)
            .ok_or_else(|| io::Error::other("missing fresh reservation"))?
            .generation;
        assert!(fresh.spawn_target.is_some());
        assert!(second_generation.0 > first_generation.0);
        Ok(())
    }

    #[test]
    fn stale_generation_cannot_replace_a_new_destination() -> TestResult {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("generation.png");
        let mut pending = PendingScreenshotCaptures::default();
        begin_frame(&mut pending);
        handle(&mut pending, &path, Some("old"))?;
        let stale = ready_completion(&pending, &path, b"stale")?;
        let old_identity = identity(&path, Some("old"))?;
        pending
            .captures
            .get_mut(&old_identity)
            .ok_or_else(|| io::Error::other("missing old capture"))?
            .deadline = Instant::now();
        handle(&mut pending, &path, Some("old"))?;
        finish_frame(&mut pending);
        assert!(
            pending
                .handle(request(&path, Some("old"))?, target()?)
                .is_err()
        );
        let acknowledgement = ready_completion(&pending, &path, b"discarded")?;
        pending.begin_frame(vec![acknowledgement]);
        finish_frame(&mut pending);
        begin_frame(&mut pending);
        finish_frame(&mut pending);

        begin_frame(&mut pending);
        handle(&mut pending, &path, Some("new"))?;
        fs::write(&path, b"current")?;
        pending.begin_frame(vec![stale]);
        handle(&mut pending, &path, Some("new"))?;
        finish_frame(&mut pending);

        assert_eq!(fs::read(path)?, b"current");
        Ok(())
    }

    #[test]
    fn real_watcher_publishes_once_and_tombstone_waits_for_receiver_close() -> TestResult {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("live-watcher.png");
        let mut app = remote_app();
        let response_receiver = send_remote_request(&app, &path, "live")?;
        app.update();

        let completion = {
            let pending = app.world().resource::<PendingScreenshotCaptures>();
            ready_completion(pending, &path, b"complete png")?
        };
        app.world()
            .resource::<CaptureCompletionChannel>()
            .sender
            .send(completion)?;

        app.update();
        assert!(!path.exists());
        app.update();
        assert!(path.exists());
        app.update();
        let terminal = receive_terminal(&response_receiver)?;
        assert_eq!(
            terminal.get(crate::constants::RESPONSE_SUCCESS_FIELD),
            Some(&Value::Bool(true))
        );

        app.update();
        assert!(matches!(
            response_receiver.try_recv(),
            Err(TryRecvError::Empty)
        ));
        app.update();
        assert!(matches!(
            response_receiver.try_recv(),
            Err(TryRecvError::Empty)
        ));
        {
            let pending = app.world().resource::<PendingScreenshotCaptures>();
            assert_eq!(pending.captures.len(), 1);
            assert_eq!(pending.reservations.len(), 1);
            assert!(pending.target_batches.is_empty());
        }
        let mut screenshots = app.world_mut().query_filtered::<Entity, With<Screenshot>>();
        assert_eq!(screenshots.iter(app.world()).count(), 1);

        drop(response_receiver);
        app.update();
        assert_eq!(
            app.world()
                .resource::<PendingScreenshotCaptures>()
                .captures
                .len(),
            1
        );
        app.update();
        let pending = app.world().resource::<PendingScreenshotCaptures>();
        assert!(pending.captures.is_empty());
        assert!(pending.reservations.is_empty());
        Ok(())
    }

    #[test]
    fn real_disconnected_watcher_suppresses_publication() -> TestResult {
        let temp_dir = TempDir::new()?;
        let path = temp_dir.path().join("closed-watcher.png");
        let mut app = remote_app();
        let response_receiver = send_remote_request(&app, &path, "closed")?;
        app.update();

        let completion = {
            let pending = app.world().resource::<PendingScreenshotCaptures>();
            ready_completion(pending, &path, b"must not publish")?
        };
        app.world()
            .resource::<CaptureCompletionChannel>()
            .sender
            .send(completion)?;
        drop(response_receiver);

        app.update();
        assert!(!path.exists());
        app.update();
        assert!(!path.exists());
        assert!(
            app.world()
                .resource::<PendingScreenshotCaptures>()
                .captures
                .is_empty()
        );
        Ok(())
    }
}
