use bevy::{
    prelude::{Sphere, *},
    render::view::RenderLayers,
};

use super::ModeSpecific;
use crate::{
    gizmos::{
        ConnectionPlane, ConnectionPoint, MaterialIndicatesSelection, Selectable,
        SelectionMaterials,
    },
    state::{EditorMode, EditorViewMode},
};
use mines::render_layer;

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

pub fn spawn_connection_plane(
    commands: &mut Commands,
    materials: &SelectionMaterials,
    meshes: &mut ResMut<Assets<Mesh>>,
    transform: Transform,
) {
    commands
        .spawn((
            RenderLayers::from_layers(&[render_layer::EDITOR_PREVIEW]),
            ModeSpecific(EditorMode::Tunnels, Some(EditorViewMode::Preview)),
            ConnectionPlane,
            RayCastBackfaces,
            transform,
            Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(1.0, 0.125, 1.0)))),
            MeshMaterial3d(materials.unselected.clone()),
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
            MeshMaterial3d(materials.unselected.clone()),
        ));

    commands.spawn((
        RenderLayers::from_layers(&[render_layer::EDITOR_PREVIEW]),
        ModeSpecific(EditorMode::Tunnels, Some(EditorViewMode::Preview)),
        ConnectionPoint,
        Transform::from_translation(transform.translation * Vec3::new(0.4, 1.0, 0.0)),
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.unselected.clone()),
        MaterialIndicatesSelection,
        Selectable,
    ));
}
