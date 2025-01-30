use bevy::{
    asset::RenderAssetUsages,
    ecs::system::SystemState,
    pbr::wireframe::Wireframe,
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        view::RenderLayers,
    },
};
use uuid::Uuid;

use crate::{
    data::{RoomPart, RoomPartPayload, RoomPartUuid},
    gizmos::{
        MaterialIndicatesSelection, PortalGizmos, Selectable, SelectionMaterials,
        SelectionWireframeColors, WireframeIndicatesSelection,
    },
    mode::ModeSpecific,
    state::{EditorMode, EditorState, FilePayload},
};
use lib::render_layer;

pub struct SpawnRoomPartEditorBundle(pub Uuid);

impl Command for SpawnRoomPartEditorBundle {
    fn apply(self, world: &mut World) {
        let mut system_state: SystemState<(
            Commands,
            ResMut<Assets<Mesh>>,
            Res<SelectionMaterials>,
            Res<SelectionWireframeColors>,
            Res<EditorState>,
        )> = SystemState::new(world);
        let (mut commands, mut meshes, materials, wireframes, state) = system_state.get_mut(world);

        let Some(data) = state.files.current_data() else {
            return;
        };
        let FilePayload::Room(data) = data else {
            return;
        };
        let Some(part) = data.parts.get(&self.0) else {
            return;
        };
        let RoomPart {
            uuid,
            transform,
            data,
        } = part;

        match data {
            RoomPartPayload::Stl {
                vertices,
                indices,
                geometry_hash,
                ..
            } => {
                let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
                    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone())
                    .with_inserted_indices(Indices::U32(indices.clone()));

                commands.spawn((
                    ModeSpecific(EditorMode::Rooms, None),
                    RenderLayers::from_layers(&[render_layer::EDITOR]),
                    RoomPartUuid(*uuid, Some(*geometry_hash)),
                    Selectable,
                    WireframeIndicatesSelection,
                    Wireframe,
                    wireframes.unselected(),
                    Mesh3d(meshes.add(mesh)),
                    *transform,
                ));
            }
            RoomPartPayload::Portal => {
                commands.spawn((
                    ModeSpecific(EditorMode::Rooms, None),
                    RenderLayers::from_layers(&[render_layer::EDITOR]),
                    RoomPartUuid(*uuid, None),
                    PortalGizmos,
                    Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(1.0, 1.0, 1.0)))),
                    materials.unselected(),
                    MaterialIndicatesSelection,
                    Selectable,
                    *transform,
                ));
            }
        };

        system_state.apply(world);
    }
}
