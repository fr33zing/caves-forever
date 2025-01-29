use std::collections::HashSet;

use bevy::{
    asset::{Assets, RenderAssetUsages},
    color::Color,
    pbr::wireframe::{Wireframe, WireframeColor},
    prelude::{
        Bundle, Changed, Commands, Component, Entity, Mesh, Mesh3d, Query, Res, ResMut, Transform,
    },
    render::{
        mesh::{Indices, PrimitiveTopology},
        view::RenderLayers,
    },
    time::Time,
};
use egui::{menu, Align, ComboBox, Frame, Label, Layout, RichText, ScrollArea, Ui};
use strum::IntoEnumIterator;
use uuid::Uuid;

use super::ModeSpecific;
use crate::{
    gizmos::Pickable,
    state::{EditorMode, EditorState, EditorViewMode, FilePayload},
};
use mines::worldgen::{
    asset::{Environment, Rarity, RoomPart, RoomPartPayload, RoomPartUuid},
    brush::TerrainBrush,
};

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
            commands.spawn(room_part_to_editor_bundle(part, &mut meshes));
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
    terrain_brushes: Query<(Entity, &TerrainBrush)>,
) {
    let Some(data) = state.files.current_data() else {
        return;
    };
    let FilePayload::Room(data) = data else {
        return;
    };

    const TIMER_SECS: f64 = 0.5;

    let mut clear_brushes = Vec::<Uuid>::new();

    update_preview_brushes.iter().for_each(|(upb_entity, upb)| {
        if time.elapsed_secs_f64() - upb.time < TIMER_SECS {
            return;
        }

        let Some(part) = data.parts.get(&upb.uuid) else {
            todo!();
        };

        clear_brushes.push(upb.uuid);
        commands.entity(upb_entity).clear();
        commands.spawn(part.to_brush_request());
    });

    clear_brushes.into_iter().for_each(|uuid| {
        let uuid = uuid.to_string();

        terrain_brushes.iter().for_each(|(entity, brush)| {
            if brush.uuid() == &uuid {
                commands.entity(entity).despawn();
            }
        });
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

pub fn sidebar(state: &mut EditorState, ui: &mut Ui) {
    let picker = &mut state.files;
    let Some(file) = picker.current_file_mut() else {
        return;
    };
    let Some(ref mut data) = file.data else {
        return;
    };
    let FilePayload::Room(data) = data else {
        todo!();
    };

    ui.style_mut().spacing.item_spacing.y = 8.0;

    ui.add(Label::new(RichText::new("Room").heading()).selectable(false));

    // Environment
    ui.columns_const(|[left, right]| {
        left.add(Label::new("Environment").selectable(false));
        right.with_layout(Layout::right_to_left(Align::Min), |right| {
            ComboBox::from_id_salt("room_environment")
                .selected_text(format!("{}", data.environment))
                .show_ui(right, |ui| {
                    Environment::iter().for_each(|env| {
                        ui.selectable_value(&mut data.environment, env, format!("{env}"));
                    });
                });
        });
    });

    // Rarity
    ui.columns_const(|[left, right]| {
        left.add(Label::new("Rarity").selectable(false));
        right.with_layout(Layout::right_to_left(Align::Min), |right| {
            ComboBox::from_id_salt("room_rarity")
                .selected_text(format!("{}", data.rarity))
                .show_ui(right, |ui| {
                    Rarity::iter().for_each(|rarity| {
                        ui.selectable_value(&mut data.rarity, rarity, format!("{rarity}"));
                    });
                });
        });
    });

    ui.separator();

    // Point
    ScrollArea::vertical().show(ui, |ui| {
        if let Some(selection_index) = state.tunnels_mode.selected_point {
            ui.add(
                Label::new(RichText::new(format!("Point {selection_index}")).heading())
                    .selectable(false),
            );

        } else {
            ui.add(Label::new(RichText::new("Point").heading()).selectable(false));
            ui.add(Label::new("No point selected.").selectable(false));
        }
    });
}

//
// Utility
//

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
            vertices, indices, ..
        } => {
            let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone())
                .with_inserted_indices(Indices::U32(indices.clone()));

            (
                ModeSpecific(EditorMode::Rooms, None),
                RenderLayers::from_layers(&[1]),
                RoomPartUuid(*uuid),
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
