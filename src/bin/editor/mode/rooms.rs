use std::collections::HashSet;

use bevy::{
    asset::RenderAssetUsages,
    pbr::wireframe::{Wireframe, WireframeColor},
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        view::RenderLayers,
    },
};
use egui::{menu, Frame, Ui};
use mines::worldgen::{
    asset::{RoomPart, RoomPartPayload, RoomPartUuid},
    brush::TerrainBrush,
};
use uuid::Uuid;

use crate::{
    gizmos::Pickable,
    state::{EditorMode, EditorState, EditorViewMode, FilePayload},
};

use super::ModeSpecific;

#[derive(Component)]
pub struct UpdatePreviewBrush {
    time: f64,
    uuid: Uuid,
}

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
    time: Res<Time>,
    mut commands: Commands,
    mut state: ResMut<EditorState>,
    parts: Query<(&Transform, &RoomPartUuid), Changed<Transform>>,
    update_preview_brushes: Query<(Entity, &UpdatePreviewBrush)>,
) {
    let Some(data) = state.files.current_data_mut() else {
        return;
    };
    let FilePayload::Room(data) = data else {
        return;
    };

    let mut update_uuids = Vec::<Uuid>::new();

    parts.iter().for_each(|(transform, uuid)| {
        let Some(part) = data.parts.get_mut(&uuid.0) else {
            return;
        };

        part.transform = *transform;
        update_uuids.push(uuid.0);
    });

    for (entity, update_preview_brush) in update_preview_brushes.into_iter() {
        if update_uuids.contains(&update_preview_brush.uuid) {
            commands.entity(entity).clear();
        }
    }

    for uuid in update_uuids {
        commands.spawn(UpdatePreviewBrush {
            time: time.elapsed_secs_f64(),
            uuid,
        });
    }
}

// Hook: update
pub fn update_preview_brushes(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<EditorState>,
    update_preview_brushes: Query<(Entity, &UpdatePreviewBrush)>,
    terrain_brushes: Query<Entity, With<TerrainBrush>>,
) {
    let Some(data) = state.files.current_data() else {
        return;
    };
    let FilePayload::Room(data) = data else {
        return;
    };

    const TIMER_SECS: f64 = 0.5;

    let mut clear_brushes = false;

    update_preview_brushes.iter().for_each(|(upb_entity, upb)| {
        if time.elapsed_secs_f64() - upb.time < TIMER_SECS {
            return;
        }

        let Some(part) = data.parts.get(&upb.uuid) else {
            todo!();
        };

        clear_brushes = true;
        commands.entity(upb_entity).clear();
        commands.spawn(part.to_brush_request());
    });

    if clear_brushes {
        terrain_brushes.iter().for_each(|entity| {
            commands.entity(entity).despawn();
        });
    }
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
                ModeSpecific(EditorMode::Rooms, None),
                RenderLayers::from_layers(&[1]),
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
