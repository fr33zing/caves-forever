use bevy::{
    math::{EulerRot, Quat, Vec3},
    prelude::{Single, Transform, With},
};
use egui::{
    menu, Align, CollapsingHeader, ComboBox, Frame, Label, Layout, RichText, ScrollArea, Ui,
};
use strum::{EnumProperty, IntoEnumIterator};

use crate::{
    gizmos::PrimarySelection,
    state::{EditorState, EditorViewMode, FilePayload},
    ui::vhacd_parameters_sidebar,
};
use lib::worldgen::asset::{Environment, Rarity, RoomPart, RoomPartPayload, RoomPartUuid};

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
                        if ui.selectable_label(false, "Portal").clicked() {
                            ui.close_menu();
                            data.push(RoomPart::portal(
                                Transform::from_scale(Vec3::new(10.0, 1.0, 10.0)).with_rotation(
                                    Quat::from_euler(
                                        EulerRot::YXZ,
                                        -90.0_f32.to_radians(),
                                        -90.0_f32.to_radians(),
                                        0.0,
                                    ),
                                ),
                            ));
                        };
                    });
                });
            });
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

        ui.add(Label::new(RichText::new("Selection").heading()).selectable(false));

        let part_name = part.data.get_str("name").unwrap();
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

                vhacd_parameters_sidebar(ui, vhacd_parameters);

                if reload {
                    // TODO handle error
                    part.reload_stl().unwrap();
                }
            }
            _ => {}
        }
    });
}
