use egui::{menu, Align, ComboBox, Frame, Label, Layout, RichText, ScrollArea, Ui};
use lib::worldgen::asset::{Environment, Rarity};
use strum::IntoEnumIterator;

use crate::state::{EditorState, EditorViewMode, FilePayload};

pub fn topbar(state: &mut EditorState, ui: &mut Ui) {
    let Some(data) = state.files.current_data_mut() else {
        return;
    };
    let FilePayload::Tunnel(data) = data else {
        todo!();
    };

    match state.view {
        EditorViewMode::Editor => {
            Frame::none().show(ui, |ui| {
                ui.shrink_width_to_current();
                menu::bar(ui, |ui| {
                    ui.menu_button("Operations", |ui| {
                        if ui
                            .selectable_label(false, "Center on world origin")
                            .clicked()
                        {
                            ui.close_menu();
                            data.center();
                        };
                    });
                });
            });

            ui.checkbox(&mut state.tunnels_mode.mirror, "Mirror");
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
    let FilePayload::Tunnel(data) = data else {
        todo!();
    };

    ui.style_mut().spacing.item_spacing.y = 8.0;

    ui.add(Label::new(RichText::new("Tunnel").heading()).selectable(false));

    // Environment
    ui.columns_const(|[left, right]| {
        left.add(Label::new("Environment").selectable(false));
        right.with_layout(Layout::right_to_left(Align::Min), |right| {
            ComboBox::from_id_salt("tunnel_environment")
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
            ComboBox::from_id_salt("tunnel_rarity")
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

            let selection = &data.points[selection_index];
            ui.add(
                Label::new(format!(
                    "{selection_index}: ({}, {})",
                    selection.position.x, selection.position.y
                ))
                .selectable(false),
            );
        } else {
            ui.add(Label::new(RichText::new("Point").heading()).selectable(false));
            ui.add(Label::new("No point selected.").selectable(false));
        }
    });
}
