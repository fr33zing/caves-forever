use bevy::{
    prelude::{Sphere, *},
    render::view::RenderLayers,
};

use super::ModeSpecific;
use crate::{
    gizmos::{ConnectionPoint, PortalGizmos},
    picking::{MaterialIndicatesSelection, Selectable, SelectionMaterials},
    state::{EditorMode, EditorViewMode},
};
use lib::render_layer;

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
            Selectable { order: 0 },
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
        Transform::from_translation(transform.translation - transform.rotation * Vec3::Y * 10.0),
        Mesh3d(meshes.add(Sphere::new(0.5))),
        materials.unselected(),
        MaterialIndicatesSelection,
        Selectable { order: 0 },
    ));
}
