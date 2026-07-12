//! Pending screenshot watcher lifecycle and PNG publication.

#[cfg(test)]
mod tests;

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

use super::CaptureInput;
use super::CaptureTarget;
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
use crate::screenshot::CaptureResponseMetadata;
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
    fingerprint:       RequestFingerprint,
    generation:        PathGeneration,
    owner:             CaptureIdentity,
    response_metadata: CaptureResponseMetadata,
    watcher_ids:       HashSet<CaptureIdentity>,
    worker_deadline:   Instant,
    work:              ReservationWork,
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
    jobs:              Vec<ScreenshotJob>,
    screenshot_entity: Option<Entity>,
}

struct PublicationClaim {
    capture:         OwnedTempCapture,
    path:            PathBuf,
    path_generation: PathGeneration,
}

pub(super) struct CaptureDispatch {
    pub(super) response:     Option<Value>,
    pub(super) spawn_target: Option<CaptureTarget>,
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
        capture_input: CaptureInput,
    ) -> BrpResult<CaptureDispatch> {
        self.handle_at(request, capture_input, Instant::now())
    }

    pub(super) fn join_existing(
        &mut self,
        request: &ScreenshotRequest,
    ) -> Option<BrpResult<CaptureDispatch>> {
        self.join_existing_at(request, Instant::now())
    }

    fn join_existing_at(
        &mut self,
        request: &ScreenshotRequest,
        now: Instant,
    ) -> Option<BrpResult<CaptureDispatch>> {
        if let Some(dispatch) = self.read_existing(request) {
            return Some(dispatch);
        }

        let reservation = self.reservations.get_mut(request.path())?;
        if now >= reservation.worker_deadline && reservation.work.owns_incomplete_generation() {
            return Some(Err(expired_generation_error(request.path())));
        }
        if &reservation.fingerprint != request.fingerprint() {
            return Some(Err(destination_conflict_error(request.path())));
        }

        let path = request.path().to_path_buf();
        let identity = request.identity().clone();
        reservation.watcher_ids.insert(identity.clone());
        let (state, result, error) = watcher_state(&reservation.work);
        self.captures.insert(
            identity,
            CaptureRecord {
                deadline: reservation.worker_deadline,
                delivery_status: DeliveryStatus::Awaiting,
                error,
                fingerprint: reservation.fingerprint.clone(),
                path,
                path_generation: reservation.generation,
                result,
                seen_frame: self.current_frame,
                state,
            },
        );
        Some(Ok(CaptureDispatch {
            response:     None,
            spawn_target: None,
        }))
    }

    fn handle_at(
        &mut self,
        request: ScreenshotRequest,
        capture_input: CaptureInput,
        now: Instant,
    ) -> BrpResult<CaptureDispatch> {
        if let Some(dispatch) = self.join_existing_at(&request, now) {
            return dispatch;
        }

        let (path, fingerprint, identity, _) = request.into_parts();
        let CaptureInput {
            crop,
            normalized_target,
            render_target,
            response_metadata,
        } = capture_input;

        let path_generation = self.next_path_generation(&path)?;
        let deadline = now + SCREENSHOT_CAPTURE_DEADLINE;
        let screenshot_job = ScreenshotJob {
            path: path.clone(),
            crop,
            identity: identity.clone(),
            path_generation,
            deadline,
            response_metadata: response_metadata.clone(),
        };
        let spawn_target = if let Some(batch) = self.target_batches.get_mut(&normalized_target) {
            batch.jobs.push(screenshot_job);
            None
        } else {
            self.target_batches.insert(
                normalized_target.clone(),
                TargetBatch {
                    jobs:              vec![screenshot_job],
                    screenshot_entity: None,
                },
            );
            Some(CaptureTarget {
                normalized_target,
                render_target,
            })
        };

        self.reservations.insert(
            path.clone(),
            DestinationReservation {
                fingerprint: fingerprint.clone(),
                generation: path_generation,
                owner: identity.clone(),
                response_metadata,
                watcher_ids: HashSet::from([identity.clone()]),
                worker_deadline: deadline,
                work: ReservationWork::Capturing,
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

    fn advance(&mut self, now: Instant) -> Vec<Entity> {
        self.update_watcher_liveness(now);
        let expired_screenshot_entities = self.expire_capturing_jobs(now);
        let claims = self.claim_publications();
        for claim in claims {
            self.publish(claim);
        }
        expired_screenshot_entities
    }

    fn expire_capturing_jobs(&mut self, now: Instant) -> Vec<Entity> {
        let mut expired_jobs = Vec::new();
        for batch in self.target_batches.values_mut() {
            batch.jobs.retain(|job| {
                if now >= job.deadline {
                    expired_jobs.push((
                        job.path.clone(),
                        job.path_generation,
                        job.identity.clone(),
                    ));
                    false
                } else {
                    true
                }
            });
        }

        let mut screenshot_entities = Vec::new();
        self.target_batches.retain(|_, batch| {
            if batch.jobs.is_empty() {
                if let Some(screenshot_entity) = batch.screenshot_entity {
                    screenshot_entities.push(screenshot_entity);
                }
                false
            } else {
                true
            }
        });

        for (path, path_generation, identity) in expired_jobs {
            let has_active_watcher = self.reservation_has_active_watcher(&path);
            let Some(reservation) = self.reservations.get_mut(&path) else {
                continue;
            };
            if reservation.generation != path_generation
                || reservation.owner != identity
                || !matches!(reservation.work, ReservationWork::Capturing)
            {
                continue;
            }

            if has_active_watcher {
                let error = timeout_error();
                reservation.work = ReservationWork::Failed(error.clone());
                self.timeout_active_watchers(&path, error);
            } else {
                reservation.work = ReservationWork::Suppressed;
            }
        }

        screenshot_entities
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
                let response = screenshot::completed_response(
                    &claim.path,
                    &claim.capture.metadata.response_metadata,
                );
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
                        && reservation.response_metadata == claim.capture.metadata.response_metadata
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

    fn record_screenshot_entity(
        &mut self,
        target: &NormalizedRenderTarget,
        screenshot_entity: Entity,
    ) {
        if let Some(batch) = self.target_batches.get_mut(target) {
            batch.screenshot_entity = Some(screenshot_entity);
        }
    }
}

pub(super) fn spawn_target_batch(world: &mut World, target: CaptureTarget) {
    let observer_target = target.normalized_target;
    let batch_target = observer_target.clone();
    let screenshot_entity = world
        .spawn((
            Screenshot(target.render_target),
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
        )
        .id();
    world
        .resource_mut::<PendingScreenshotCaptures>()
        .record_screenshot_entity(&batch_target, screenshot_entity);
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

pub(super) fn advance_capture_lifecycle(
    mut commands: Commands,
    mut pending: ResMut<PendingScreenshotCaptures>,
) {
    let expired_screenshot_entities = pending.advance(Instant::now());
    for screenshot_entity in expired_screenshot_entities {
        commands.entity(screenshot_entity).try_despawn();
    }
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
