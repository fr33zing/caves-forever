use std::{path::PathBuf, str::FromStr};

use egui::{
    Align, Button, Color32, Frame, Label, Layout, Margin, RichText, Rounding, ScrollArea, Sense,
    Stroke, Ui, UiBuilder,
};

use crate::state::EditorState;

use super::{icons, SaveAsDialogState};

pub fn file_browser(state: &mut EditorState, ui: &mut Ui) {
    ui.style_mut().spacing.item_spacing.y = 0.0;

    Frame::none()
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(Label::new("Filter:").selectable(false));
                ui.text_edit_singleline(&mut state.tunnels_mode.files.filter);
            });
        });

    ui.separator();

    ScrollArea::vertical().show(ui, |ui| {
        ui.style_mut().spacing.item_spacing.y = 0.0;

        let Some(picker) = state.file_picker_mut() else {
            return;
        };

        let current = picker.current;
        let filter = picker.filter.trim();
        let mut index_to_open: Option<usize> = None;

        // TODO This is gonna be slow. Sorry.
        let mut sorted = picker.files.iter().enumerate().collect::<Vec<_>>();
        sorted.sort_by_key(|(_, file)| (file.changed, file.modified_time));
        sorted.reverse();

        let mut row_i = 0; // For alternative bg colors
        for (file_i, file) in sorted.into_iter() {
            if !filter.is_empty() && !file.name.contains(filter) {
                continue;
            }

            let response = ui
                .scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                    let response = ui.response();
                    let is_current_file = file_i == current;

                    let bg_fill = if row_i % 2 == 0 {
                        Color32::TRANSPARENT
                    } else {
                        Color32::from_gray(35)
                    };

                    let bg_fill_interactive = if response.clicked() {
                        Color32::from_gray(70)
                    } else if response.hovered() {
                        Color32::from_gray(50)
                    } else {
                        bg_fill
                    };

                    Frame::canvas(ui.style())
                        .fill(bg_fill_interactive)
                        .stroke(Stroke::NONE)
                        .rounding(Rounding::ZERO)
                        .inner_margin(Margin::symmetric(8.0, 4.0))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal_wrapped(|ui| {
                                let mut filename = RichText::new(file.name.clone());
                                if is_current_file {
                                    filename = filename.color(Color32::from_rgb(50, 200, 200));
                                }

                                ui.add(Label::new(filename).selectable(false));
                                ui.add_space(ui.available_size_before_wrap().x - 8.0);
                                if file.changed {
                                    icons::changed_default(ui);
                                }
                            });
                        });
                })
                .response;

            if response.clicked() {
                index_to_open = Some(file_i);
            }

            row_i += 1;
        }

        if let Some(i) = index_to_open {
            picker.switch_to_file(i).unwrap(); // TODO handle this
        }
    });
}

pub fn save_as_dialog(dialog: &mut SaveAsDialogState, ui: &mut Ui) -> (bool, bool) {
    let mut close_dialog = false;
    let mut write_file = false;

    ui.style_mut().spacing.item_spacing.y = 8.0;

    ui.horizontal(|ui| {
        ui.add(Label::new("Name:").selectable(false));
        ui.text_edit_singleline(&mut dialog.filename);
    });

    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
        let save_button = ui.add(Button::new("Save").fill(Color32::from_rgb(45, 100, 45)));
        if save_button.clicked() {
            write_file = true;
            close_dialog = true;
            return;
        };

        let cancel_button = ui.add(Button::new("Cancel").fill(Color32::from_rgb(100, 45, 45)));
        if cancel_button.clicked() {
            close_dialog = true;
            return;
        };
    });

    return (close_dialog, write_file);
}
