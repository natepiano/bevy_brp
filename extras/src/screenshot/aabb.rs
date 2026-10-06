//! AABB entity visibility validation and physical crop projection, for one entity or the visible
//! descendants of an entity without its own `Aabb`.

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
use bevy_remote::BrpError;
use bevy_remote::BrpResult;
use bevy_remote::error_codes::INVALID_PARAMS;

use super::BoundsKind;
use super::SelectedCamera;

pub(super) struct ResolvedBounds {
    pub(super) bounds_kind: BoundsKind,
    pub(super) rect:        URect,
}

/// Physical target-space footprint of one or more projected `Aabb`s.
#[derive(Clone, Copy)]
enum Projection {
    Extent(Rect),
    /// A corner crosses the near or far plane, so the footprint is the whole viewport.
    WholeViewport,
}

impl Projection {
    fn union(self, other: Self) -> Self {
        match (self, other) {
            (Self::Extent(first), Self::Extent(second)) => Self::Extent(first.union(second)),
            _ => Self::WholeViewport,
        }
    }
}

/// Whether the entity or any descendant carries an `Aabb`, checked before camera selection.
pub(super) fn has_bounds(world: &World, entity: Entity) -> bool {
    world.get::<Aabb>(entity).is_some()
        || descendants(world, entity).any(|descendant| world.get::<Aabb>(descendant).is_some())
}

pub(super) fn resolve(
    world: &World,
    entity: Entity,
    selected_camera: &SelectedCamera,
    padding: u32,
) -> BrpResult<ResolvedBounds> {
    if world.get::<Aabb>(entity).is_some() {
        validate_visibility(world, entity, selected_camera)?;
        let projection = project_entity(world, entity, selected_camera)?
            .ok_or_else(|| bounds_error(entity, "Aabb is outside the selected camera frustum"))?;
        return Ok(ResolvedBounds {
            bounds_kind: BoundsKind::Aabb,
            rect:        crop(projection, selected_camera, padding)?,
        });
    }

    resolve_hierarchy(world, entity, selected_camera, padding)
}

/// Unions the projections of every visible descendant with an `Aabb`. A mesh-less root never
/// gets `ViewVisibility`, so the root check covers only `Visibility` and `InheritedVisibility`.
fn resolve_hierarchy(
    world: &World,
    entity: Entity,
    selected_camera: &SelectedCamera,
    padding: u32,
) -> BrpResult<ResolvedBounds> {
    if matches!(world.get::<Visibility>(entity), Some(Visibility::Hidden))
        || world
            .get::<InheritedVisibility>(entity)
            .is_some_and(|visibility| !visibility.get())
    {
        return Err(bounds_error(entity, "is hidden"));
    }

    let mut union = None;
    for descendant in descendants(world, entity) {
        if world.get::<Aabb>(descendant).is_none()
            || world.get::<GlobalTransform>(descendant).is_none()
            || validate_visibility(world, descendant, selected_camera).is_err()
        {
            continue;
        }
        if let Some(projection) = project_entity(world, descendant, selected_camera)? {
            union = Some(union.map_or(projection, |union: Projection| union.union(projection)));
        }
    }

    let projection = union.ok_or_else(|| {
        bounds_error(
            entity,
            "has no visible descendant with an Aabb inside the selected camera frustum",
        )
    })?;
    Ok(ResolvedBounds {
        bounds_kind: BoundsKind::Hierarchy,
        rect:        crop(projection, selected_camera, padding)?,
    })
}

fn descendants(world: &World, entity: Entity) -> impl Iterator<Item = Entity> {
    let mut stack = world
        .get::<Children>(entity)
        .map(|children| children.to_vec())
        .unwrap_or_default();
    std::iter::from_fn(move || {
        let next = stack.pop()?;
        if let Some(children) = world.get::<Children>(next) {
            stack.extend(children.iter());
        }
        Some(next)
    })
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

/// Projects the entity's `Aabb`, or `None` when it lies outside the camera frustum.
fn project_entity(
    world: &World,
    entity: Entity,
    selected_camera: &SelectedCamera,
) -> BrpResult<Option<Projection>> {
    let aabb = world
        .get::<Aabb>(entity)
        .ok_or_else(|| bounds_error(entity, "does not have an Aabb component"))?;
    let global_transform = world
        .get::<GlobalTransform>(entity)
        .ok_or_else(|| bounds_error(entity, "does not have a GlobalTransform component"))?;
    if !selected_camera
        .frustum
        .intersects_obb(aabb, &global_transform.affine(), true, true)
    {
        return Ok(None);
    }

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
                return Ok(Some(Projection::WholeViewport));
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

    Ok(Some(Projection::Extent(Rect::from_corners(
        projected_min,
        projected_max,
    ))))
}

/// Pads the projected footprint once and clips it to the camera viewport and target.
fn crop(
    projection: Projection,
    selected_camera: &SelectedCamera,
    padding: u32,
) -> BrpResult<URect> {
    let viewport = selected_camera
        .camera
        .physical_viewport_rect()
        .ok_or_else(|| camera_projection_error(selected_camera.entity, "has no viewport size"))?;
    let target_size = selected_camera
        .camera
        .physical_target_size()
        .ok_or_else(|| camera_projection_error(selected_camera.entity, "has no target size"))?;
    let target = URect::from_corners(UVec2::ZERO, target_size);

    let Projection::Extent(extent) = projection else {
        return nonempty_intersection(viewport, target, selected_camera.entity);
    };
    let min = clamped_physical_point(extent.min.floor(), target_size);
    let max = clamped_physical_point(extent.max.ceil(), target_size);
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
    point.clamp(Vec2::ZERO, target_size.as_vec2()).as_uvec2()
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
        resolved(world, entity, selected_camera, padding).map(|resolved| resolved.rect)
    }

    fn resolved(
        world: &World,
        entity: Entity,
        selected_camera: &SelectedCamera,
        padding: u32,
    ) -> Result<ResolvedBounds, IoError> {
        resolve(world, entity, selected_camera, padding)
            .map_err(|error| io::Error::other(error.message))
    }

    fn child(world: &mut World, parent: Entity, x: f32) -> Entity {
        world
            .spawn((
                Aabb::from_min_max(Vec3::splat(-0.25), Vec3::splat(0.25)),
                GlobalTransform::from(Transform::from_xyz(x, 0.0, 0.5)),
                ChildOf(parent),
            ))
            .id()
    }

    fn root(world: &mut World) -> Entity { world.spawn(GlobalTransform::IDENTITY).id() }

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

    #[test]
    fn hierarchy_bounds_union_visible_descendants_and_grandchildren() -> Result<(), Box<dyn Error>>
    {
        let selected_camera = selected_camera(UVec2::splat(100), 1.0, None);
        let mut world = World::new();
        let root = root(&mut world);
        child(&mut world, root, -0.5);
        let group = world.spawn((GlobalTransform::IDENTITY, ChildOf(root))).id();
        child(&mut world, group, 0.5);

        let resolved = resolved(&world, root, &selected_camera, 0)?;

        assert_eq!(resolved.bounds_kind, BoundsKind::Hierarchy);
        assert_eq!(resolved.rect, URect::new(12, 37, 88, 63));
        assert_eq!(
            rect(&world, root, &selected_camera, 2)?,
            URect::new(10, 35, 90, 65)
        );
        Ok(())
    }

    #[test]
    fn hierarchy_bounds_skip_hidden_and_offscreen_descendants() -> Result<(), Box<dyn Error>> {
        let selected_camera = selected_camera(UVec2::splat(100), 1.0, None);
        let mut world = World::new();
        let root = root(&mut world);
        child(&mut world, root, 0.25);
        let hidden = child(&mut world, root, -0.5);
        world.entity_mut(hidden).insert(Visibility::Hidden);
        let inherited_hidden = child(&mut world, root, -0.5);
        world
            .entity_mut(inherited_hidden)
            .insert(InheritedVisibility::HIDDEN);
        child(&mut world, root, 3.0);

        assert_eq!(
            rect(&world, root, &selected_camera, 0)?,
            URect::new(50, 37, 75, 63)
        );

        world.entity_mut(root).insert(Visibility::Hidden);
        assert!(resolve(&world, root, &selected_camera, 0).is_err());
        world
            .entity_mut(root)
            .insert((Visibility::Inherited, InheritedVisibility::HIDDEN));
        assert!(resolve(&world, root, &selected_camera, 0).is_err());
        Ok(())
    }

    #[test]
    fn hierarchy_without_visible_bounds_is_rejected() {
        let selected_camera = selected_camera(UVec2::splat(100), 1.0, None);
        let mut world = World::new();
        let bare = root(&mut world);
        world.spawn((GlobalTransform::IDENTITY, ChildOf(bare)));
        let offscreen = root(&mut world);
        child(&mut world, offscreen, 3.0);

        assert!(!has_bounds(&world, bare));
        assert!(has_bounds(&world, offscreen));
        assert!(matches!(
            resolve(&world, offscreen, &selected_camera, 0),
            Err(error) if error.message.contains("has no visible descendant")
        ));
    }
}
