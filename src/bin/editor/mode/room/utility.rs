use bevy::{
    asset::RenderAssetUsages,
    pbr::wireframe::{Wireframe, WireframeColor},
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        view::RenderLayers,
    },
};
use mines::{
    render_layer,
    worldgen::asset::{RoomPart, RoomPartPayload, RoomPartUuid},
};

use crate::{
    gizmos::{Selectable, WireframeIndicatesSelection},
    mode::ModeSpecific,
    state::EditorMode,
};

pub fn room_part_to_editor_bundle(
    room_part: &RoomPart,
    meshes: &mut ResMut<Assets<Mesh>>,
) -> impl Bundle {
    let RoomPart {
        uuid,
        transform,
        data,
    } = room_part;

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

            (
                ModeSpecific(EditorMode::Rooms, None),
                RenderLayers::from_layers(&[render_layer::EDITOR]),
                RoomPartUuid(*uuid, Some(*geometry_hash)),
                Selectable,
                WireframeIndicatesSelection,
                Wireframe,
                WireframeColor {
                    color: Color::WHITE,
                },
                Mesh3d(meshes.add(mesh)),
                transform.to_owned(),
            )
        }
    }
}
