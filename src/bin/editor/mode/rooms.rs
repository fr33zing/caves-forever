use std::collections::HashSet;

use bevy::{
    asset::RenderAssetUsages,
    pbr::wireframe::{Wireframe, WireframeColor},
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use egui::{menu, Frame, Ui};
use mines::worldgen::asset::{RoomPart, RoomPartPayload, RoomPartUuid};
use transform_gizmo_bevy::GizmoTarget;
use uuid::Uuid;

use crate::{
    gizmos::Pickable,
    state::{EditorState, EditorViewMode, FilePayload},
};

//
// Systems
//

// Hook: update
pub fn detect_additions(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    state: Res<EditorState>,
    parts: Query<&RoomPartUuid>,
) {
    let Some(data) = state.files.current_data() else {
        return;
    };
    let FilePayload::Room(data) = data else {
        return;
    };

    let mut existing = HashSet::<Uuid>::new();
    parts.iter().for_each(|uuid| {
        existing.insert(uuid.0);
    });

    data.parts.iter().for_each(|(uuid, part)| {
        if !existing.contains(uuid) {
            commands.spawn(room_part_to_editor_bundle(part, *uuid, &mut meshes));
        }
    });
}

// Hook: update
pub fn detect_world_changes(
    mut state: ResMut<EditorState>,
    parts: Query<(&Transform, &RoomPartUuid), Changed<Transform>>,
) {
    let Some(data) = state.files.current_data_mut() else {
        return;
    };
    let FilePayload::Room(data) = data else {
        return;
    };

    parts.iter().for_each(|(transform, uuid)| {
        let Some(part) = data.parts.get_mut(&uuid.0) else {
            return;
        };

        part.transform = *transform;
    });
}

// HACK there should probably be a proper revert event
// Hook: update
pub fn detect_file_changes(
    mut state: ResMut<EditorState>,
    mut parts: Query<(&mut Transform, &RoomPartUuid)>,
    gizmo_targets: Query<(Entity, &GizmoTarget)>,
) {
    if gizmo_targets.iter().any(|(_, target)| target.is_focused()) {
        return;
    }
    let Some(data) = state.files.current_data_mut() else {
        return;
    };
    let FilePayload::Room(data) = data else {
        return;
    };

    parts.iter_mut().for_each(|(mut transform, uuid)| {
        let Some(part) = data.parts.get_mut(&uuid.0) else {
            return;
        };

        *transform = part.transform;
    });
}

//
// UI
//

pub fn topbar(state: &mut EditorState, ui: &mut Ui) {
    let Some(data) = state.files.current_data_mut() else {
        return;
    };
    let FilePayload::Room(data) = data else {
        todo!();
    };

    match state.view {
        EditorViewMode::Editor => {
            Frame::none().show(ui, |ui| {
                ui.shrink_width_to_current();
                menu::bar(ui, |ui| {
                    ui.menu_button("Add", |ui| {
                        if ui.selectable_label(false, "STL Import").clicked() {
                            ui.close_menu();
                            data.push(RoomPart::default_stl(Transform::default()).unwrap());
                        };
                    });
                });
            });
        }
        EditorViewMode::Preview => {}
    }
}

//
// Utility
//

pub fn room_part_to_editor_bundle(
    room_part: &RoomPart,
    uuid: Uuid,
    meshes: &mut ResMut<Assets<Mesh>>,
) -> impl Bundle {
    let RoomPart { transform, data } = room_part;

    match data {
        RoomPartPayload::Stl {
            vertices, indices, ..
        } => {
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone())
                .with_inserted_indices(Indices::U32(indices.clone()));

            (
                RoomPartUuid(uuid),
                Pickable(None, None),
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
