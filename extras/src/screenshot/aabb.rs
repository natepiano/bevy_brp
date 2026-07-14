//! AABB entity visibility validation and physical crop projection.

use bevy::camera::ViewportConversionError;
use bevy::camera::primitives::Aabb;
use bevy::camera::visibility::DEFAULT_LAYERS;
use bevy::camera::visibility::InheritedVisibility;
use bevy::camera::visibility::NoCpuCulling;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::visibility::ViewVisibility;
use bevy::camera::visibility::Visibility;
use bevy::camera::visibility::VisibilityClass;
use bevy::prelude::*;
use bevy_kana::ToF32;
use bevy_kana::ToU32;
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INVALID_PARAMS;

use super::SelectedCamera;

pub(super) fn resolve(
    world: &World,
    entity: Entity,
    selected_camera: &SelectedCamera,
    padding: u32,
) -> BrpResult<URect> {
    validate_visibility(world, entity, selected_camera)?;
    let aabb = world
        .get::<Aabb>(entity)
        .ok_or_else(|| bounds_error(entity, "does not have an Aabb component"))?;
    let global_transform = world
        .get::<GlobalTransform>(entity)
        .ok_or_else(|| bounds_error(entity, "does not have a GlobalTransform component"))?;

    project_aabb(entity, aabb, global_transform, selected_camera, padding)
}

fn validate_visibility(
    world: &World,
    entity: Entity,
    selected_camera: &SelectedCamera,
) -> BrpResult<()> {
    if matches!(world.get::<Visibility>(entity), Some(Visibility::Hidden))
        || world
            .get::<InheritedVisibility>(entity)
            .is_some_and(|visibility| !visibility.get())
        || world
            .get::<ViewVisibility>(entity)
            .is_some_and(|visibility| !visibility.get())
    {
        return Err(bounds_error(entity, "is hidden"));
    }

    let entity_layers = world.get::<RenderLayers>(entity).unwrap_or(DEFAULT_LAYERS);
    let camera_layers = selected_camera
        .render_layers
        .as_ref()
        .unwrap_or(DEFAULT_LAYERS);
    if !entity_layers.intersects(camera_layers) {
        return Err(bounds_error(
            entity,
            "does not share a RenderLayers entry with the selected camera",
        ));
    }

    if world.get::<NoCpuCulling>(entity).is_some() {
        return Ok(());
    }

    if let (Some(visibility_class), Some(visible_entities)) = (
        world.get::<VisibilityClass>(entity),
        selected_camera.visible_entities.as_ref(),
    ) && !visibility_class.is_empty()
        && !visibility_class
            .iter()
            .any(|class| visible_entities.get(*class).contains(&entity))
    {
        return Err(bounds_error(
            entity,
            "is not visible from the selected camera",
        ));
    }

    Ok(())
}

fn project_aabb(
    entity: Entity,
    aabb: &Aabb,
    global_transform: &GlobalTransform,
    selected_camera: &SelectedCamera,
    padding: u32,
) -> BrpResult<URect> {
    if !selected_camera
        .frustum
        .intersects_obb(aabb, &global_transform.affine(), true, true)
    {
        return Err(bounds_error(
            entity,
            "Aabb is outside the selected camera frustum",
        ));
    }

    let viewport = selected_camera
        .camera
        .physical_viewport_rect()
        .ok_or_else(|| camera_projection_error(selected_camera.entity, "has no viewport size"))?;
    let target_size = selected_camera
        .camera
        .physical_target_size()
        .ok_or_else(|| camera_projection_error(selected_camera.entity, "has no target size"))?;
    let target = URect::from_corners(UVec2::ZERO, target_size);
    let scaling_factor = selected_camera
        .camera
        .target_scaling_factor()
        .filter(|factor| factor.is_finite() && *factor > 0.0)
        .ok_or_else(|| {
            camera_projection_error(selected_camera.entity, "has an invalid target scale")
        })?;

    let mut projected_min = Vec2::splat(f32::INFINITY);
    let mut projected_max = Vec2::splat(f32::NEG_INFINITY);
    for corner in aabb_corners(aabb) {
        let world_corner = global_transform.transform_point(corner);
        match selected_camera
            .camera
            .world_to_viewport(&selected_camera.global_transform, world_corner)
        {
            Ok(logical) => {
                let physical = logical * scaling_factor;
                if !physical.is_finite() {
                    return Err(camera_projection_error(
                        selected_camera.entity,
                        "produced non-finite viewport coordinates",
                    ));
                }
                projected_min = projected_min.min(physical);
                projected_max = projected_max.max(physical);
            },
            Err(ViewportConversionError::PastNearPlane | ViewportConversionError::PastFarPlane) => {
                return nonempty_intersection(viewport, target, selected_camera.entity);
            },
            Err(ViewportConversionError::NoViewportSize) => {
                return Err(camera_projection_error(
                    selected_camera.entity,
                    "has no viewport size",
                ));
            },
            Err(ViewportConversionError::InvalidData) => {
                return Err(camera_projection_error(
                    selected_camera.entity,
                    "has invalid projection data",
                ));
            },
        }
    }

    let min = clamped_physical_point(projected_min.floor(), target_size);
    let max = clamped_physical_point(projected_max.ceil(), target_size);
    let padding = UVec2::splat(padding);
    let padded = URect::from_corners(min.saturating_sub(padding), max.saturating_add(padding));

    nonempty_intersection(padded.intersect(viewport), target, selected_camera.entity)
}

fn aabb_corners(aabb: &Aabb) -> [Vec3; 8] {
    let center = Vec3::from(aabb.center);
    let half_extents = Vec3::from(aabb.half_extents);
    [
        center + half_extents * Vec3::new(-1.0, -1.0, -1.0),
        center + half_extents * Vec3::new(-1.0, -1.0, 1.0),
        center + half_extents * Vec3::new(-1.0, 1.0, -1.0),
        center + half_extents * Vec3::new(-1.0, 1.0, 1.0),
        center + half_extents * Vec3::new(1.0, -1.0, -1.0),
        center + half_extents * Vec3::new(1.0, -1.0, 1.0),
        center + half_extents * Vec3::new(1.0, 1.0, -1.0),
        center + half_extents,
    ]
}

fn clamped_physical_point(point: Vec2, target_size: UVec2) -> UVec2 {
    UVec2::new(
        point.x.clamp(0.0, target_size.x.to_f32()).to_u32(),
        point.y.clamp(0.0, target_size.y.to_f32()).to_u32(),
    )
}

fn nonempty_intersection(rect: URect, hard_bounds: URect, camera: Entity) -> BrpResult<URect> {
    let intersection = rect.intersect(hard_bounds);
    if intersection.is_empty() {
        return Err(camera_projection_error(camera, "produced an empty crop"));
    }
    Ok(intersection)
}

fn bounds_error(entity: Entity, detail: &str) -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: format!("Screenshot entity {} {detail}", entity.to_bits()),
        data:    None,
    }
}

fn camera_projection_error(camera: Entity, detail: &str) -> BrpError {
    BrpError {
        code:    INVALID_PARAMS,
        message: format!("Screenshot camera {} {detail}", camera.to_bits()),
        data:    None,
    }
}

#[cfg(test)]
mod tests {
    use std::any::TypeId;
    use std::error::Error;
    use std::io;
    use std::io::Error as IoError;

    use bevy::camera::ComputedCameraValues;
    use bevy::camera::RenderTarget;
    use bevy::camera::RenderTargetInfo;
    use bevy::camera::Viewport;
    use bevy::camera::primitives::Frustum;
    use bevy::camera::visibility::VisibleEntities;
    use bevy::math::primitives::ViewFrustum;
    use bevy::window::WindowRef;

    use super::*;

    #[derive(Component)]
    struct TestVisibilityClass;

    fn selected_camera(
        target_size: UVec2,
        scale_factor: f32,
        viewport: Option<Viewport>,
    ) -> SelectedCamera {
        let render_target = RenderTarget::Window(WindowRef::Primary);
        let clip_from_view = Mat4::IDENTITY;
        SelectedCamera {
            camera: Camera {
                computed: ComputedCameraValues {
                    clip_from_view,
                    target_info: Some(RenderTargetInfo {
                        physical_size: target_size,
                        scale_factor,
                    }),
                    ..default()
                },
                viewport,
                ..default()
            },
            entity: Entity::PLACEHOLDER,
            frustum: Frustum(ViewFrustum::from_clip_from_world(&clip_from_view)),
            global_transform: GlobalTransform::IDENTITY,
            render_layers: None,
            render_target,
            visible_entities: None,
        }
    }

    fn entity(world: &mut World, global_transform: GlobalTransform) -> Entity {
        world
            .spawn((
                Aabb::from_min_max(Vec3::splat(-0.25), Vec3::splat(0.25)),
                global_transform,
            ))
            .id()
    }

    fn rect(
        world: &World,
        entity: Entity,
        selected_camera: &SelectedCamera,
        padding: u32,
    ) -> Result<URect, IoError> {
        resolve(world, entity, selected_camera, padding)
            .map_err(|error| io::Error::other(error.message))
    }

    #[test]
    fn transformed_aabbs_produce_physical_containing_rectangles() -> Result<(), Box<dyn Error>> {
        let selected_camera = selected_camera(UVec2::splat(100), 1.0, None);
        let mut world = World::new();
        let translated = entity(
            &mut world,
            GlobalTransform::from(Transform::from_xyz(0.25, 0.0, 0.5)),
        );
        let rotated = entity(
            &mut world,
            GlobalTransform::from(
                Transform::from_xyz(0.0, 0.0, 0.5)
                    .with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_4)),
            ),
        );
        let scaled = entity(
            &mut world,
            GlobalTransform::from(
                Transform::from_xyz(0.0, 0.0, 0.5).with_scale(Vec3::new(2.0, 0.5, 1.0)),
            ),
        );
        let reflected = entity(
            &mut world,
            GlobalTransform::from(
                Transform::from_xyz(0.0, 0.0, 0.5).with_scale(Vec3::new(-1.0, 1.0, 1.0)),
            ),
        );

        assert_eq!(
            rect(&world, translated, &selected_camera, 0)?,
            URect::new(50, 37, 75, 63)
        );
        assert_eq!(
            rect(&world, scaled, &selected_camera, 0)?,
            URect::new(25, 43, 75, 57)
        );
        assert_eq!(
            rect(&world, reflected, &selected_camera, 0)?,
            URect::new(37, 37, 63, 63)
        );
        let rotated = rect(&world, rotated, &selected_camera, 0)?;
        assert_eq!(rotated.min, UVec2::splat(32));
        assert_eq!(rotated.max, UVec2::splat(68));
        Ok(())
    }

    #[test]
    fn scaling_viewport_and_padding_use_physical_hard_bounds() -> Result<(), Box<dyn Error>> {
        let viewport = Viewport {
            physical_position: UVec2::new(10, 20),
            physical_size: UVec2::new(60, 40),
            ..default()
        };
        let selected_camera = selected_camera(UVec2::splat(100), 2.0, Some(viewport));
        let mut world = World::new();
        let entity = entity(
            &mut world,
            GlobalTransform::from(Transform::from_xyz(-0.75, 0.0, 0.5)),
        );

        let crop = rect(&world, entity, &selected_camera, 20)?;

        assert_eq!(crop, URect::new(10, 20, 45, 60));
        Ok(())
    }

    #[test]
    fn near_and_far_plane_crossings_use_the_complete_viewport() -> Result<(), Box<dyn Error>> {
        let viewport = Viewport {
            physical_position: UVec2::new(10, 20),
            physical_size: UVec2::new(60, 40),
            ..default()
        };
        let selected_camera = selected_camera(UVec2::splat(100), 1.0, Some(viewport));
        let mut world = World::new();
        let near = entity(
            &mut world,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.9)),
        );
        let far = entity(
            &mut world,
            GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.1)),
        );

        assert_eq!(
            rect(&world, near, &selected_camera, 0)?,
            URect::new(10, 20, 70, 60)
        );
        assert_eq!(
            rect(&world, far, &selected_camera, 0)?,
            URect::new(10, 20, 70, 60)
        );
        Ok(())
    }

    #[test]
    fn offscreen_hidden_and_disjoint_layer_entities_are_rejected() {
        let selected_camera = selected_camera(UVec2::splat(100), 1.0, None);
        let mut world = World::new();
        let offscreen = entity(
            &mut world,
            GlobalTransform::from(Transform::from_xyz(3.0, 0.0, 0.5)),
        );
        let hidden = world
            .spawn((
                Aabb::from_min_max(Vec3::splat(-0.25), Vec3::splat(0.25)),
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.5)),
                Visibility::Hidden,
            ))
            .id();
        let disjoint = world
            .spawn((
                Aabb::from_min_max(Vec3::splat(-0.25), Vec3::splat(0.25)),
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.5)),
                RenderLayers::layer(1),
            ))
            .id();

        assert!(resolve(&world, offscreen, &selected_camera, 0).is_err());
        assert!(resolve(&world, hidden, &selected_camera, 0).is_err());
        assert!(resolve(&world, disjoint, &selected_camera, 0).is_err());
    }

    #[test]
    fn selected_view_membership_applies_except_for_no_cpu_culling() -> Result<(), Box<dyn Error>> {
        let mut selected_camera = selected_camera(UVec2::splat(100), 1.0, None);
        let mut world = World::new();
        let mut visibility_class = VisibilityClass::default();
        visibility_class.push(TypeId::of::<TestVisibilityClass>());
        let culled = world
            .spawn((
                Aabb::from_min_max(Vec3::splat(-0.25), Vec3::splat(0.25)),
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.5)),
                visibility_class.clone(),
            ))
            .id();
        let uncullable = world
            .spawn((
                Aabb::from_min_max(Vec3::splat(-0.25), Vec3::splat(0.25)),
                GlobalTransform::from(Transform::from_xyz(0.0, 0.0, 0.5)),
                visibility_class,
                NoCpuCulling,
            ))
            .id();
        selected_camera.visible_entities = Some(VisibleEntities::default());

        assert!(resolve(&world, culled, &selected_camera, 0).is_err());
        assert!(resolve(&world, uncullable, &selected_camera, 0).is_ok());
        selected_camera
            .visible_entities
            .as_mut()
            .ok_or_else(|| io::Error::other("missing visible entity table"))?
            .push(culled, TypeId::of::<TestVisibilityClass>());
        assert!(resolve(&world, culled, &selected_camera, 0).is_ok());
        Ok(())
    }
}
