use bevy::{
    math::{EulerRot, Quat, Vec3},
    prelude::{Single, Transform, With},
};
use egui::{
    menu, Align, CollapsingHeader, ComboBox, Frame, Label, Layout, RichText, ScrollArea, Ui,
};
use lib::worldgen::asset::PortalDirection;
use strum::{EnumProperty, IntoEnumIterator};

use crate::{
    data::{Environment, Rarity, RoomPart, RoomPartPayload, RoomPartUuid},
    picking::PrimarySelection,
    state::{EditorState, EditorViewMode, FilePayload},
    ui::vhacd_parameters_sidebar,
};

pub fn topbar(state: &mut EditorState, ui: &mut Ui) {
    let Some(data) = state.files.current_data_mut() else {
        return;
    };
    let FilePayload::Room(data) = data else {
        todo!();
    };

    match state.view {
        EditorViewMode::Editor => {
            // Add menu
            let mut add: Option<RoomPart> = None;
            Frame::none().show(ui, |ui| {
                ui.shrink_width_to_current();
                menu::bar(ui, |ui| {
                    ui.menu_button("Add", |ui| {
                        if ui.selectable_label(false, "STL Import").clicked() {
                            ui.close_menu();
                            add = Some(RoomPart::default_stl(Transform::default()).unwrap());
                        };

                        ui.menu_button("Portal", |ui| {
                            let transform = Transform::from_scale(Vec3::new(10.0, 1.0, 10.0))
                                .with_rotation(Quat::from_euler(
                                    EulerRot::YXZ,
                                    -90.0_f32.to_radians(),
                                    -90.0_f32.to_radians(),
                                    0.0,
                                ));

                            PortalDirection::iter().for_each(|direction| {
                                if ui.selectable_label(false, direction.to_string()).clicked() {
                                    add = Some(RoomPart::portal(transform, direction));
                                }
                            });
                        });
                    });
                });
            });
            if let Some(mut add) = add {
                add.place_after_spawn = true;
                data.push(add);
            }
        }
        EditorViewMode::Preview => {}
    }
}

pub fn sidebar(
    state: &mut EditorState,
    ui: &mut Ui,
    selected: Option<Single<&RoomPartUuid, With<PrimarySelection>>>,
) {
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

    // Selection
    ScrollArea::vertical().show(ui, |ui| {
        let Some(selected) = selected else {
            return;
        };
        let selected_uuid = selected.into_inner();
        let Some(part) = data.parts.get_mut(&selected_uuid.0) else {
            todo!()
        };
        let Some(part_name) = part.data.get_str("name") else {
            return;
        };

        ui.add(Label::new(RichText::new("Selection").heading()).selectable(false));

        match &mut part.data {
            RoomPartPayload::Stl {
                path,
                vhacd_parameters,
                ..
            } => {
                let mut reload = false;

                CollapsingHeader::new(part_name)
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.text_edit_singleline(path);
                        ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                            if ui.button("Load").clicked() {
                                reload = true;
                            }
                            if ui.button("Browse").clicked() {}
                        });
                    });

                let vhacd_changed = vhacd_parameters_sidebar(ui, vhacd_parameters);

                // TODO handle errors
                if reload {
                    part.reload_stl().unwrap();
                } else if vhacd_changed {
                    part.rehash_stl().unwrap();
                }
            }
            RoomPartPayload::Portal { direction } => {
                CollapsingHeader::new(part_name)
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.columns_const(|[left, right]| {
                            left.add(Label::new("Direction").selectable(false));
                            right.with_layout(Layout::right_to_left(Align::Min), |right| {
                                ComboBox::from_id_salt("portal_direction")
                                    .selected_text(direction.to_string())
                                    .show_ui(right, |ui| {
                                        PortalDirection::iter().for_each(|dir| {
                                            ui.selectable_value(direction, dir, dir.to_string());
                                        });
                                    });
                            });
                        });
                    });
            }
        }
    });
}
