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
    gizmos::{PortalGizmos, SpawnpointGizmos},
    mode::ModeSpecific,
    picking::{
        MaterialIndicatesSelection, Selectable, SelectionMaterials, SelectionWireframeColors,
        SpawnAndPlaceCommand, WireframeIndicatesSelection,
    },
    state::{EditorMode, EditorState, FilePayload},
};
use lib::{
    player::consts::{PLAYER_HEIGHT, PLAYER_RADIUS},
    render_layer,
};

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

        let placement = part.placement();
        let RoomPart {
            uuid,
            transform,
            data,
            place_after_spawn,
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
                let bundle = (
                    ModeSpecific(EditorMode::Rooms, None),
                    RenderLayers::from_layers(&[render_layer::EDITOR]),
                    RoomPartUuid(*uuid, Some(*geometry_hash)),
                    Selectable,
                    WireframeIndicatesSelection,
                    Wireframe,
                    wireframes.unselected(),
                    Mesh3d(meshes.add(mesh)),
                    *transform,
                );
                if *place_after_spawn {
                    commands.queue(SpawnAndPlaceCommand {
                        modes: placement,
                        offset: Vec3::ZERO,
                        align_to_hit_normal: false,
                        bundle,
                    });
                } else {
                    commands.spawn(bundle);
                }
            }
            RoomPartPayload::Portal { .. } => {
                let bundle = (
                    ModeSpecific(EditorMode::Rooms, None),
                    RenderLayers::from_layers(&[render_layer::EDITOR]),
                    RoomPartUuid(*uuid, None),
                    PortalGizmos,
                    Mesh3d(meshes.add(Cuboid::from_size(Vec3::new(1.0, 1.0, 1.0)))),
                    materials.unselected(),
                    MaterialIndicatesSelection,
                    Selectable,
                    *transform,
                );
                if *place_after_spawn {
                    commands.queue(SpawnAndPlaceCommand {
                        modes: placement,
                        offset: Vec3::ZERO,
                        align_to_hit_normal: true,
                        bundle,
                    });
                } else {
                    commands.spawn(bundle);
                }
            }
            RoomPartPayload::Spawnpoint => {
                let bundle = (
                    ModeSpecific(EditorMode::Rooms, None),
                    RenderLayers::from_layers(&[render_layer::EDITOR]),
                    RoomPartUuid(*uuid, None),
                    SpawnpointGizmos,
                    Mesh3d(meshes.add(Capsule3d::new(
                        PLAYER_RADIUS,
                        (PLAYER_HEIGHT - PLAYER_RADIUS * 2.0) / 2.0,
                    ))),
                    materials.unselected(),
                    MaterialIndicatesSelection,
                    Selectable,
                    *transform,
                );
                if *place_after_spawn {
                    commands.queue(SpawnAndPlaceCommand {
                        modes: placement,
                        offset: Vec3::Y * PLAYER_HEIGHT / 2.0,
                        align_to_hit_normal: false,
                        bundle,
                    });
                } else {
                    commands.spawn(bundle);
                }
            }
        };

        system_state.apply(world);
    }
}
