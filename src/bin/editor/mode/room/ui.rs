use bevy::prelude::Transform;
use egui::{menu, Align, ComboBox, Frame, Label, Layout, RichText, ScrollArea, Ui};
use strum::IntoEnumIterator;

use crate::state::{EditorState, EditorViewMode, FilePayload};
use mines::worldgen::asset::{Environment, Rarity, RoomPart};

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

    // Selection
    ScrollArea::vertical().show(ui, |ui| {});
}
