//! Bevy UI entity bounds and target resolution.

use bevy::camera::visibility::InheritedVisibility;
use bevy::math::Affine2;
use bevy::math::Rect;
use bevy::prelude::*;
use bevy::ui::CalculatedClip;
use bevy::ui::CalculatedClipRect;
use bevy::ui::ComputedNode;
use bevy::ui::ComputedUiRenderTargetInfo;
use bevy::ui::ComputedUiTargetCamera;
use bevy::ui::UiGlobalTransform;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INVALID_PARAMS;

use super::ValidatedCameraTarget;
use super::primary_window;
use super::validated_camera_target;

pub(super) struct ResolvedUiCapture {
    pub(super) camera: ValidatedCameraTarget,
    pub(super) rect:   URect,
}

enum UiFamily<'a> {
    Absent,
    Complete {
        computed_node:                  &'a ComputedNode,
        computed_ui_render_target_info: &'a ComputedUiRenderTargetInfo,
        computed_ui_target_camera:      &'a ComputedUiTargetCamera,
        ui_global_transform:            &'a UiGlobalTransform,
    },
    Partial,
}

pub(super) fn resolve(
    world: &World,
    entity: Entity,
    requested_camera: Option<Entity>,
    padding: u32,
) -> BrpResult<Option<ResolvedUiCapture>> {
    let (
        computed_node,
        computed_ui_render_target_info,
        computed_ui_target_camera,
        ui_global_transform,
    ) = match classify(world, entity) {
        UiFamily::Absent => return Ok(None),
        UiFamily::Complete {
            computed_node,
            computed_ui_render_target_info,
            computed_ui_target_camera,
            ui_global_transform,
        } => (
            computed_node,
            computed_ui_render_target_info,
            computed_ui_target_camera,
            ui_global_transform,
        ),
        UiFamily::Partial => {
            return Err(ui_error(entity, "has partially initialized UI bounds"));
        },
    };

    let inherited_visibility = world
        .get::<InheritedVisibility>(entity)
        .ok_or_else(|| ui_error(entity, "has no initialized inherited visibility"))?;
    if !inherited_visibility.get() {
        return Err(ui_error(entity, "is hidden"));
    }

    let camera_entity = computed_ui_target_camera
        .get()
        .ok_or_else(|| ui_error(entity, "has no initialized UI target camera"))?;
    if requested_camera.is_some_and(|requested| requested != camera_entity) {
        return Err(ui_error(
            entity,
            "targets a different camera than the requested camera",
        ));
    }

    let primary_window = primary_window(world);
    let camera = validated_camera_target(world, camera_entity, primary_window)
        .ok_or_else(|| invalid_ui_camera_error(entity, camera_entity))?;
    let rect = transformed_rect(
        entity,
        computed_node,
        ui_global_transform,
        computed_ui_render_target_info,
        world.get::<CalculatedClip>(entity),
        &camera,
        padding,
    )?;

    Ok(Some(ResolvedUiCapture { camera, rect }))
}

fn classify(world: &World, entity: Entity) -> UiFamily<'_> {
    match (
        world.get::<ComputedNode>(entity),
        world.get::<UiGlobalTransform>(entity),
        world.get::<ComputedUiTargetCamera>(entity),
        world.get::<ComputedUiRenderTargetInfo>(entity),
    ) {
        (None, None, None, None) => UiFamily::Absent,
        (
            Some(computed_node),
            Some(ui_global_transform),
            Some(computed_ui_target_camera),
            Some(computed_ui_render_target_info),
        ) => UiFamily::Complete {
            computed_node,
            computed_ui_render_target_info,
            computed_ui_target_camera,
            ui_global_transform,
        },
        _ => UiFamily::Partial,
    }
}

fn transformed_rect(
    entity: Entity,
    computed_node: &ComputedNode,
    ui_global_transform: &UiGlobalTransform,
    computed_ui_render_target_info: &ComputedUiRenderTargetInfo,
    calculated_clip: Option<&CalculatedClip>,
    camera: &ValidatedCameraTarget,
    padding: u32,
) -> BrpResult<URect> {
    let size = computed_node.size();
    if !size.is_finite() || !size.cmpgt(Vec2::ZERO).all() {
        return Err(ui_error(entity, "has invalid or empty computed dimensions"));
    }

    let affine = ui_global_transform.affine();
    let node_polygon = rect_corners(Rect::from_center_size(Vec2::ZERO, size))
        .map(|corner| affine.transform_point2(corner));
    if node_polygon.iter().any(|corner| !corner.is_finite()) {
        return Err(ui_error(entity, "produced non-finite transformed bounds"));
    }

    let local_target = Rect::from_corners(
        Vec2::ZERO,
        computed_ui_render_target_info.physical_size().as_vec2(),
    );
    let clip_rects = validated_clip_rects(entity, calculated_clip)?;
    let target_region = CalculatedClipRect {
        rect:                local_target,
        world_to_clip_local: Affine2::IDENTITY,
    };
    let outside_error = || ui_error(entity, "is outside its UI viewport or clip");
    let local_clip = polygon_bounds(&clip_polygon(rect_corners(local_target), clip_rects))
        .ok_or_else(outside_error)?;
    let local_rect = polygon_bounds(&clip_polygon(
        node_polygon,
        clip_rects.iter().chain([&target_region]),
    ))
    .ok_or_else(outside_error)?;

    let viewport = camera
        .camera
        .physical_viewport_rect()
        .ok_or_else(|| invalid_ui_camera_error(entity, camera.entity))?;
    let viewport_offset = viewport.min.as_vec2();
    let rect = containing_rect(local_rect.translate(viewport_offset))?;
    let translated_clip = containing_rect(local_clip.translate(viewport_offset))?;
    let padding = UVec2::splat(padding);
    let padded = URect::from_corners(
        rect.min.saturating_sub(padding),
        rect.max.saturating_add(padding),
    );
    let target = URect::from_corners(UVec2::ZERO, camera.target_size);
    let rect = padded
        .intersect(translated_clip)
        .intersect(viewport)
        .intersect(target);
    if rect.is_empty() {
        return Err(ui_error(entity, "produced an empty crop"));
    }

    Ok(rect)
}

/// Returns the clip regions inherited through `CalculatedClip`, empty when the node is unclipped.
///
/// An infinite `CalculatedClipRect::rect` edge is the representation Bevy uses for an
/// `OverflowAxis::Visible` axis, so it means "unbounded" and is accepted; NaN edges and
/// non-finite `world_to_clip_local` transforms are rejected.
fn validated_clip_rects(
    entity: Entity,
    calculated_clip: Option<&CalculatedClip>,
) -> BrpResult<&[CalculatedClipRect]> {
    let Some(calculated_clip) = calculated_clip else {
        return Ok(&[]);
    };
    let clip_rects = calculated_clip
        .rects()
        .ok_or_else(|| ui_error(entity, "is fully clipped"))?;
    if clip_rects
        .iter()
        .any(|clip_rect| clip_rect.rect.min.is_nan() || clip_rect.rect.max.is_nan())
    {
        return Err(ui_error(entity, "has NaN clip coordinates"));
    }
    if clip_rects
        .iter()
        .any(|clip_rect| !clip_rect.world_to_clip_local.is_finite())
    {
        return Err(ui_error(entity, "has a non-finite clip transform"));
    }
    Ok(clip_rects)
}

/// Corners of `rect` in boundary order, as `clip_polygon` requires.
const fn rect_corners(rect: Rect) -> [Vec2; 4] {
    [
        rect.min,
        Vec2::new(rect.max.x, rect.min.y),
        rect.max,
        Vec2::new(rect.min.x, rect.max.y),
    ]
}

/// Clips a convex polygon against every finite edge of each `CalculatedClipRect` with the
/// Sutherland-Hodgman algorithm, matching `bevy::ui_render::clipping::clip_polygon`.
fn clip_polygon<'a>(
    polygon: impl Into<Vec<Vec2>>,
    clip_rects: impl IntoIterator<Item = &'a CalculatedClipRect>,
) -> Vec<Vec2> {
    let mut visible = polygon.into();
    for clip_rect in clip_rects {
        let edges = [
            (-clip_rect.rect.min.x, Vec2::X),
            (clip_rect.rect.max.x, Vec2::NEG_X),
            (clip_rect.rect.max.y, Vec2::NEG_Y),
            (-clip_rect.rect.min.y, Vec2::Y),
        ];
        for (edge, inward_normal) in edges {
            if !edge.is_finite() {
                continue;
            }
            let distance = |point: Vec2| {
                clip_rect
                    .world_to_clip_local
                    .transform_point2(point)
                    .dot(inward_normal)
                    + edge
            };
            let Some(&last) = visible.last() else {
                return visible;
            };
            let mut clipped = Vec::with_capacity(visible.len() + 1);
            let mut previous = last;
            let mut previous_distance = distance(previous);
            for &vertex in &visible {
                let vertex_distance = distance(vertex);
                if (previous_distance >= 0.0) != (vertex_distance >= 0.0) {
                    let t = previous_distance / (previous_distance - vertex_distance);
                    clipped.push(previous.lerp(vertex, t));
                }
                if vertex_distance >= 0.0 {
                    clipped.push(vertex);
                }
                previous = vertex;
                previous_distance = vertex_distance;
            }
            visible = clipped;
        }
    }
    visible
}

/// Axis-aligned bounds of a clipped polygon, or `None` when nothing with area remains.
fn polygon_bounds(polygon: &[Vec2]) -> Option<Rect> {
    if polygon.len() < 3 {
        return None;
    }
    let (min, max) = polygon.iter().fold(
        (Vec2::splat(f32::INFINITY), Vec2::splat(f32::NEG_INFINITY)),
        |(min, max), &vertex| (min.min(vertex), max.max(vertex)),
    );
    let bounds = Rect { min, max };
    (!bounds.is_empty()).then_some(bounds)
}

fn containing_rect(rect: Rect) -> BrpResult<URect> {
    if !rect.min.is_finite() || !rect.max.is_finite() {
        return Err(BrpError {
            code:    INVALID_PARAMS,
            message: "UI screenshot bounds contain non-finite coordinates".to_string(),
            data:    None,
        });
    }
    Ok(URect::from_corners(
        rect.min.floor().as_uvec2(),
        rect.max.ceil().as_uvec2(),
    ))
}

fn ui_error(entity: Entity, detail: &str) -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: format!("Screenshot UI entity {} {detail}", entity.to_bits()),
        data:    None,
    }
}

fn invalid_ui_camera_error(entity: Entity, camera: Entity) -> BrpError {
    ui_error(
        entity,
        &format!(
            "targets camera {}, which is missing, inactive, uninitialized, or has an unsupported target",
            camera.to_bits()
        ),
    )
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::f32::consts::FRAC_PI_4;
    use std::io;
    use std::io::Error as IoError;

    use bevy::app::HierarchyPropagatePlugin;
    use bevy::app::PostUpdate;
    use bevy::app::PropagateSet;
    use bevy::app::Update;
    use bevy::asset::RenderAssetUsages;
    use bevy::camera::ComputedCameraValues;
    use bevy::camera::RenderTarget;
    use bevy::camera::RenderTargetInfo;
    use bevy::camera::Viewport;
    use bevy::camera::primitives::Aabb;
    use bevy::camera::visibility::RenderLayers;
    use bevy::render::render_resource::Extent3d;
    use bevy::render::render_resource::TextureDimension;
    use bevy::render::render_resource::TextureFormat;
    use bevy::ui::Node;
    use bevy::ui::UiScale;
    use bevy::ui::UiTargetCamera;
    use bevy::ui::update::propagate_ui_target_cameras;
    use bevy::window::WindowRef;
    use bevy::window::WindowResolution;

    use super::*;
    use crate::constants::RESPONSE_BOUNDS_KIND_FIELD;
    use crate::constants::RESPONSE_NAME_FIELD;
    use crate::constants::SCREENSHOT_BOUNDS_KIND_UI;
    use crate::screenshot;
    use crate::screenshot::BoundsKind;
    use crate::screenshot::CaptureResponseMetadata;

    struct TestUi {
        app:    App,
        camera: Entity,
        window: Entity,
    }

    fn ui_app() -> App {
        let mut app = App::new();
        app.init_resource::<UiScale>()
            .add_plugins(HierarchyPropagatePlugin::<ComputedUiTargetCamera>::new(
                PostUpdate,
            ))
            .configure_sets(
                PostUpdate,
                PropagateSet::<ComputedUiTargetCamera>::default(),
            )
            .add_plugins(HierarchyPropagatePlugin::<ComputedUiRenderTargetInfo>::new(
                PostUpdate,
            ))
            .configure_sets(
                PostUpdate,
                PropagateSet::<ComputedUiRenderTargetInfo>::default(),
            )
            .add_systems(Update, propagate_ui_target_cameras);
        app
    }

    fn test_ui(target_size: UVec2, viewport: Option<Viewport>) -> TestUi {
        let mut app = ui_app();
        let window = app
            .world_mut()
            .spawn(Window {
                resolution: WindowResolution::new(target_size.x, target_size.y),
                ..default()
            })
            .id();
        let camera = app
            .world_mut()
            .spawn((
                Camera {
                    computed: ComputedCameraValues {
                        target_info: Some(RenderTargetInfo {
                            physical_size: target_size,
                            scale_factor:  1.0,
                        }),
                        ..default()
                    },
                    viewport,
                    ..default()
                },
                RenderTarget::Window(WindowRef::Entity(window)),
            ))
            .id();
        TestUi {
            app,
            camera,
            window,
        }
    }

    fn image_test_ui(target_size: UVec2) -> TestUi {
        let mut app = ui_app();
        app.init_resource::<Assets<Image>>();
        let image = Image::new_fill(
            Extent3d {
                width:                 target_size.x,
                height:                target_size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        let image_handle = app.world_mut().resource_mut::<Assets<Image>>().add(image);
        let camera = app
            .world_mut()
            .spawn((
                Camera {
                    computed: ComputedCameraValues {
                        target_info: Some(RenderTargetInfo {
                            physical_size: target_size,
                            scale_factor:  1.0,
                        }),
                        ..default()
                    },
                    ..default()
                },
                RenderTarget::Image(image_handle.into()),
            ))
            .id();
        TestUi {
            app,
            camera,
            window: Entity::PLACEHOLDER,
        }
    }

    fn spawn_node(test_ui: &mut TestUi, size: Vec2, affine: Affine2) -> Entity {
        let entity = test_ui
            .app
            .world_mut()
            .spawn((
                Node::default(),
                ComputedNode { size, ..default() },
                UiGlobalTransform::from(affine),
                UiTargetCamera(test_ui.camera),
                InheritedVisibility::VISIBLE,
            ))
            .id();
        test_ui.app.update();
        entity
    }

    fn calculated_clip(rect: Rect, world_to_clip_local: Affine2) -> CalculatedClip {
        let mut calculated_clip = CalculatedClip::default();
        calculated_clip.push_rect(rect, world_to_clip_local);
        calculated_clip
    }

    fn resolved(
        test_ui: &mut TestUi,
        entity: Entity,
        requested_camera: Option<Entity>,
        padding: u32,
    ) -> Result<ResolvedUiCapture, IoError> {
        resolve(test_ui.app.world_mut(), entity, requested_camera, padding)
            .map_err(|error| io::Error::other(error.message))?
            .ok_or_else(|| io::Error::other("entity was not classified as UI"))
    }

    fn resolution_error(result: BrpResult<Option<ResolvedUiCapture>>) -> Result<BrpError, IoError> {
        match result {
            Ok(_) => Err(io::Error::other(
                "UI bounds resolution unexpectedly succeeded",
            )),
            Err(error) => Ok(error),
        }
    }

    #[test]
    fn transformed_bounds_are_physical_containing_rectangles() -> Result<(), Box<dyn Error>> {
        let mut test_ui = test_ui(UVec2::splat(100), None);
        let affine = Affine2::from_scale_angle_translation(
            Vec2::new(2.0, 0.5),
            std::f32::consts::FRAC_PI_2,
            Vec2::new(30.25, 20.75),
        );
        let entity = spawn_node(&mut test_ui, Vec2::new(10.0, 20.0), affine);

        let resolved = resolved(&mut test_ui, entity, None, 0)?;

        assert_eq!(resolved.rect, URect::new(25, 10, 36, 31));
        Ok(())
    }

    #[test]
    fn clip_and_viewport_translation_are_hard_bounds() -> Result<(), Box<dyn Error>> {
        let viewport = Viewport {
            physical_position: UVec2::new(10, 20),
            physical_size: UVec2::new(60, 40),
            ..default()
        };
        let mut test_ui = test_ui(UVec2::new(100, 80), Some(viewport));
        let entity = spawn_node(
            &mut test_ui,
            Vec2::new(20.0, 10.0),
            Affine2::from_translation(Vec2::new(30.0, 20.0)),
        );
        test_ui
            .app
            .world_mut()
            .entity_mut(entity)
            .insert(calculated_clip(
                Rect::new(25.25, 18.25, 35.25, 24.25),
                Affine2::IDENTITY,
            ));

        let resolved = resolved(&mut test_ui, entity, None, 20)?;

        assert_eq!(resolved.rect, URect::new(35, 38, 46, 45));
        Ok(())
    }

    #[test]
    fn padding_stays_within_viewport_and_live_target_edges() -> Result<(), Box<dyn Error>> {
        let viewport = Viewport {
            physical_position: UVec2::new(90, 70),
            physical_size: UVec2::new(20, 20),
            ..default()
        };
        let mut test_ui = test_ui(UVec2::new(100, 80), Some(viewport));
        let entity = spawn_node(
            &mut test_ui,
            Vec2::splat(4.0),
            Affine2::from_translation(Vec2::splat(2.0)),
        );

        let resolved = resolved(&mut test_ui, entity, None, 20)?;

        assert_eq!(resolved.rect, URect::new(90, 70, 100, 80));
        Ok(())
    }

    #[test]
    fn non_primary_image_targets_use_the_computed_ui_camera() -> Result<(), Box<dyn Error>> {
        let mut test_ui = image_test_ui(UVec2::new(80, 60));
        let entity = spawn_node(
            &mut test_ui,
            Vec2::new(20.0, 10.0),
            Affine2::from_translation(Vec2::new(40.0, 30.0)),
        );

        let resolved = resolved(&mut test_ui, entity, None, 0)?;

        assert_eq!(resolved.camera.entity, test_ui.camera);
        assert_eq!(resolved.rect, URect::new(30, 25, 50, 35));
        Ok(())
    }

    #[test]
    fn partial_ui_and_camera_mismatch_fail_before_aabb_fallback() -> Result<(), Box<dyn Error>> {
        let mut world = World::new();
        let partial = world
            .spawn((
                ComputedNode {
                    size: Vec2::splat(10.0),
                    ..default()
                },
                Aabb::from_min_max(Vec3::splat(-0.5), Vec3::splat(0.5)),
            ))
            .id();
        let error = resolution_error(resolve(&world, partial, None, 0))?;
        assert!(error.message.contains("partially initialized UI bounds"));

        let mut test_ui = test_ui(UVec2::splat(100), None);
        let entity = spawn_node(
            &mut test_ui,
            Vec2::splat(10.0),
            Affine2::from_translation(Vec2::splat(50.0)),
        );
        let other_camera = test_ui.app.world_mut().spawn_empty().id();
        let error = resolution_error(resolve(
            test_ui.app.world_mut(),
            entity,
            Some(other_camera),
            0,
        ))?;
        assert!(error.message.contains("different camera"));
        Ok(())
    }

    #[test]
    fn missing_inherited_visibility_is_an_initialization_error() -> Result<(), Box<dyn Error>> {
        let mut test_ui = test_ui(UVec2::splat(100), None);
        let entity = spawn_node(
            &mut test_ui,
            Vec2::splat(10.0),
            Affine2::from_translation(Vec2::splat(50.0)),
        );
        test_ui
            .app
            .world_mut()
            .entity_mut(entity)
            .remove::<InheritedVisibility>();

        let error = resolution_error(resolve(test_ui.app.world_mut(), entity, None, 0))?;

        assert_eq!(error.code, INVALID_PARAMS);
        assert!(
            error
                .message
                .contains("no initialized inherited visibility")
        );
        Ok(())
    }

    #[test]
    fn rotated_clip_bounds_the_visible_polygon() -> Result<(), Box<dyn Error>> {
        let mut test_ui = test_ui(UVec2::splat(100), None);
        let world_to_clip_local =
            Affine2::from_angle_translation(FRAC_PI_4, Vec2::splat(50.0)).inverse();
        let clip = calculated_clip(Rect::new(-10.0, -10.0, 10.0, 10.0), world_to_clip_local);

        let enclosing = spawn_node(
            &mut test_ui,
            Vec2::splat(40.0),
            Affine2::from_translation(Vec2::splat(50.0)),
        );
        test_ui
            .app
            .world_mut()
            .entity_mut(enclosing)
            .insert(clip.clone());
        let resolved = resolved(&mut test_ui, enclosing, None, 0)?;
        assert_eq!(resolved.rect, URect::new(35, 35, 65, 65));

        // Inside the clip's axis-aligned bounds but outside the rotated clip itself.
        let outside_rotated_clip = spawn_node(
            &mut test_ui,
            Vec2::splat(6.0),
            Affine2::from_translation(Vec2::splat(62.0)),
        );
        test_ui
            .app
            .world_mut()
            .entity_mut(outside_rotated_clip)
            .insert(clip);
        let error = resolution_error(resolve(
            test_ui.app.world_mut(),
            outside_rotated_clip,
            None,
            0,
        ))?;
        assert!(error.message.contains("outside its UI viewport or clip"));
        Ok(())
    }

    #[test]
    fn infinite_clip_edges_are_unbounded() -> Result<(), Box<dyn Error>> {
        let mut test_ui = test_ui(UVec2::splat(100), None);
        let entity = spawn_node(
            &mut test_ui,
            Vec2::splat(20.0),
            Affine2::from_translation(Vec2::splat(50.0)),
        );
        test_ui
            .app
            .world_mut()
            .entity_mut(entity)
            .insert(calculated_clip(
                Rect {
                    min: Vec2::new(-5.0, f32::NEG_INFINITY),
                    max: Vec2::new(5.0, f32::INFINITY),
                },
                Affine2::from_translation(Vec2::splat(-50.0)),
            ));

        let resolved = resolved(&mut test_ui, entity, None, 0)?;

        assert_eq!(resolved.rect, URect::new(45, 40, 55, 60));
        Ok(())
    }

    #[test]
    fn invalid_and_full_clips_are_invalid_parameters_errors() -> Result<(), Box<dyn Error>> {
        let mut test_ui = test_ui(UVec2::splat(100), None);
        let entity = spawn_node(
            &mut test_ui,
            Vec2::splat(10.0),
            Affine2::from_translation(Vec2::splat(50.0)),
        );
        let invalid_clips = [
            (
                calculated_clip(
                    Rect {
                        min: Vec2::new(f32::NAN, 0.0),
                        max: Vec2::splat(60.0),
                    },
                    Affine2::IDENTITY,
                ),
                "NaN clip coordinates",
            ),
            (
                calculated_clip(
                    Rect::new(0.0, 0.0, 60.0, 60.0),
                    Affine2::from_translation(Vec2::new(f32::INFINITY, 0.0)),
                ),
                "non-finite clip transform",
            ),
            (CalculatedClip::FullyClipped, "is fully clipped"),
        ];

        for (clip, expected_message) in invalid_clips {
            test_ui.app.world_mut().entity_mut(entity).insert(clip);
            let error = resolution_error(resolve(test_ui.app.world_mut(), entity, None, 0))?;

            assert_eq!(error.code, INVALID_PARAMS);
            assert!(error.message.contains(expected_message));
        }
        Ok(())
    }

    #[test]
    fn ui_precedes_aabb_and_ignores_render_layers() -> Result<(), Box<dyn Error>> {
        let mut test_ui = test_ui(UVec2::splat(100), None);
        let entity = spawn_node(
            &mut test_ui,
            Vec2::splat(10.0),
            Affine2::from_translation(Vec2::splat(50.0)),
        );
        test_ui.app.world_mut().entity_mut(entity).insert((
            Aabb::from_min_max(Vec3::splat(-0.5), Vec3::splat(0.5)),
            GlobalTransform::IDENTITY,
            Name::new("Before"),
            RenderLayers::layer(1),
        ));
        test_ui
            .app
            .world_mut()
            .entity_mut(test_ui.camera)
            .insert(RenderLayers::layer(2));

        let capture_input = screenshot::entity_capture_input(
            test_ui.app.world_mut(),
            entity,
            Some(test_ui.camera),
            0,
        )
        .map_err(|error| io::Error::other(error.message))?;
        test_ui
            .app
            .world_mut()
            .entity_mut(entity)
            .insert(Name::new("After"));
        let response = screenshot::completed_response(
            std::path::Path::new("/tmp/ui-entity.png"),
            &capture_input.response_metadata,
        );

        let CaptureResponseMetadata::Entity(metadata) = capture_input.response_metadata else {
            return Err(io::Error::other("expected entity response metadata").into());
        };
        assert_eq!(metadata.bounds_kind, BoundsKind::Ui);
        assert_eq!(metadata.rect, URect::new(45, 45, 55, 55));
        assert_eq!(metadata.name.as_deref(), Some("Before"));
        assert_eq!(
            response
                .get(RESPONSE_BOUNDS_KIND_FIELD)
                .and_then(serde_json::Value::as_str),
            Some(SCREENSHOT_BOUNDS_KIND_UI)
        );
        assert_eq!(
            response
                .get(RESPONSE_NAME_FIELD)
                .and_then(serde_json::Value::as_str),
            Some("Before")
        );
        Ok(())
    }

    #[test]
    fn hidden_offscreen_and_missing_live_targets_fail() -> Result<(), Box<dyn Error>> {
        let mut test_ui = test_ui(UVec2::splat(100), None);
        let hidden = spawn_node(
            &mut test_ui,
            Vec2::splat(10.0),
            Affine2::from_translation(Vec2::splat(50.0)),
        );
        test_ui
            .app
            .world_mut()
            .entity_mut(hidden)
            .insert(InheritedVisibility::HIDDEN);
        assert!(resolve(test_ui.app.world_mut(), hidden, None, 0).is_err());

        let offscreen = spawn_node(
            &mut test_ui,
            Vec2::splat(10.0),
            Affine2::from_translation(Vec2::splat(150.0)),
        );
        assert!(resolve(test_ui.app.world_mut(), offscreen, None, 0).is_err());

        let live = spawn_node(
            &mut test_ui,
            Vec2::splat(10.0),
            Affine2::from_translation(Vec2::splat(50.0)),
        );
        test_ui.app.world_mut().despawn(test_ui.window);
        let error = resolution_error(resolve(test_ui.app.world_mut(), live, None, 0))?;
        assert!(error.message.contains("unsupported target"));
        Ok(())
    }
}
