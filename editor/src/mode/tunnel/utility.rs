use bevy::{
    prelude::{Sphere, *},
    render::view::RenderLayers,
};

use super::ModeSpecific;
use crate::{
    gizmos::{
        ConnectionPoint, MaterialIndicatesSelection, PortalGizmos, Selectable, SelectionMaterials,
    },
    state::{EditorMode, EditorViewMode},
};
use lib::render_layer;

/// Adapted from: https://bevy-cheatbook.github.io/cookbook/cursor2world.html
pub fn cursor_to_ground_plane(
    window: &Window,
    (camera, camera_transform): (&Camera, &GlobalTransform),
) -> Option<Vec2> {
    let Some(cursor_position) = window.cursor_position() else {
        return None;
    };
    let plane_origin = Vec3::ZERO;
    let plane = InfinitePlane3d::new(Vec3::Y);
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return None;
    };
    let Some(distance) = ray.intersect_plane(plane_origin, plane) else {
        return None;
    };
    let global_cursor = ray.get_point(distance);

    Some(global_cursor.xz())
}

pub fn spawn_fake_portal(
    commands: &mut Commands,
    materials: &SelectionMaterials,
    meshes: &mut ResMut<Assets<Mesh>>,
    transform: Transform,
) {
    commands
        .spawn((
            RenderLayers::from_layers(&[render_layer::EDITOR_PREVIEW]),
            ModeSpecific(EditorMode::Tunnels, Some(EditorViewMode::Preview)),
            PortalGizmos,
            RayCastBackfaces,
            transform,
            Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(1.0, 0.125, 1.0)))),
            materials.unselected(),
            MaterialIndicatesSelection,
            Selectable,
        ))
        .with_child((
            ConnectionPoint,
            Transform::from_translation(Vec3::NEG_Y * 4.0).with_scale(Vec3::new(
                1.0 / transform.scale.x,
                1.0,
                1.0 / transform.scale.z,
            )),
            materials.unselected(),
        ));

    commands.spawn((
        RenderLayers::from_layers(&[render_layer::EDITOR_PREVIEW]),
        ModeSpecific(EditorMode::Tunnels, Some(EditorViewMode::Preview)),
        ConnectionPoint,
        Transform::from_translation(transform.translation),
        Mesh3d(meshes.add(Sphere::new(0.5))),
        materials.unselected(),
        MaterialIndicatesSelection,
        Selectable,
    ));
}
