use bevy::prelude::Commands;
use egui::{
    menu, Align, Align2, Area, Button, Color32, ComboBox, Context, Frame, Id, Label, Layout,
    Margin, Response, RichText, Rounding, ScrollArea, SelectableLabel, Sense, Stroke, TextEdit, Ui,
    UiBuilder, Vec2,
};
use strum::{EnumProperty, IntoEnumIterator};

use crate::{
    mode::RevertCommand,
    state::{EditorMode, EditorState},
    ui::{open_file_action_dialog, FileActionDialogMode},
};

use super::{icons, EditorDialogVisibility, FileActionDialogState};

pub fn file_browser(
    state: &mut EditorState,
    dialogs: &mut EditorDialogVisibility,
    dialog_state: &mut FileActionDialogState,
    ui: &mut Ui,
) {
    Frame::none()
        .inner_margin(Margin::same(8.0))
        .show(ui, |ui| {
            ui.style_mut().spacing.item_spacing.y = 8.0;

            ui.columns_const(|[left, right]| {
                left.add(Label::new("Filter by name:").selectable(false));
                right.text_edit_singleline(&mut state.files.filter);
            });

            ui.columns_const(|[left, right]| {
                left.add(Label::new("Filter by mode:").selectable(false));
                let mut filter_mode_text = "All".to_owned();
                if let Some(mode) = state.files.filter_mode {
                    filter_mode_text = mode.to_string();
                }

                ComboBox::from_id_salt("filter_mode")
                    .selected_text(filter_mode_text)
                    .show_ui(right, |ui| {
                        ui.selectable_value(&mut state.files.filter_mode, None, "All");

                        EditorMode::iter().for_each(|mode| {
                            ui.selectable_value(
                                &mut state.files.filter_mode,
                                Some(mode),
                                mode.to_string(),
                            );
                        });
                    });
            });
        });

    ui.style_mut().spacing.item_spacing.y = 0.0;
    ui.separator();

    #[derive(PartialEq)]
    enum Action {
        None,
        Open,
        Revert,
        Save,
        SaveAs,
        Rename,
        Delete,
    }

    ScrollArea::vertical().show(ui, |ui| {
        ui.style_mut().spacing.item_spacing.y = 0.0;

        let current = state.files.current;
        let filter = state.files.filter.trim();
        let filter_mode = state.files.filter_mode;

        let mut action = Action::None;
        let mut index_to_act: Option<usize> = None;

        // TODO This is gonna be slow. Sorry.
        let mut sorted = state.files.files.iter().enumerate().collect::<Vec<_>>();
        sorted.sort_by_key(|(_, file)| {
            (
                file.path.is_none(),
                -(file.mode as i16),
                file.changed,
                file.modified_time,
            )
        });
        sorted.reverse();

        let mut row_i = 0; // For alternative bg colors
        for (file_i, file) in sorted.into_iter() {
            if !filter.is_empty() && !file.name.contains(filter) {
                continue;
            }
            if let Some(filter_mode) = filter_mode {
                if file.mode != filter_mode {
                    continue;
                };
            }

            let response = ui
                .scope_builder(UiBuilder::new().sense(Sense::click()), |ui| {
                    let response = ui.response();
                    let is_current_file = Some(file_i) == current;

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

                                if file.changed {
                                    icons::changed_default(ui);
                                }

                                ui.add(Label::new(filename).selectable(false));
                                ui.add_space(ui.available_size_before_wrap().x - 18.0);

                                Frame::none().show(ui, |ui| {
                                    ui.shrink_width_to_current();

                                    menu::bar(ui, |ui| {
                                        ui.menu_button("...", |ui| {
                                            ui.add(Label::new(file.name.clone()).selectable(false));

                                            ui.separator();

                                            let save_button = ui.add_enabled(
                                                file.changed,
                                                SelectableLabel::new(false, "Save"),
                                            );
                                            if save_button.clicked() {
                                                action = Action::Save;
                                            }

                                            if ui.selectable_label(false, "Save as...").clicked() {
                                                action = Action::SaveAs;
                                            }

                                            ui.separator();

                                            let revert_button = ui.add_enabled(
                                                file.changed,
                                                SelectableLabel::new(false, "Revert"),
                                            );
                                            if revert_button.clicked() {
                                                action = Action::Revert;
                                            }

                                            if ui.selectable_label(false, "Rename").clicked() {
                                                action = Action::Rename;
                                            }
                                            if ui.selectable_label(false, "Delete").clicked() {
                                                action = Action::Delete;
                                            }

                                            if action != Action::None {
                                                ui.close_menu();
                                                index_to_act = Some(file_i);
                                            }
                                        });
                                    });
                                });
                            });
                        });
                })
                .response;

            if response.clicked() {
                index_to_act = Some(file_i);
                action = Action::Open;
            }

            row_i += 1;
        }

        if let Some(file_index) = index_to_act {
            let mut open_dialog_with_mode: Option<FileActionDialogMode> = None;

            // TODO handle errors
            match action {
                Action::Open => state.files.switch_to_file(file_index).unwrap(),
                Action::Save => {
                    if !state.files.save_file(file_index).unwrap() {
                        open_dialog_with_mode = Some(FileActionDialogMode::SaveAs)
                    }
                }
                Action::SaveAs => open_dialog_with_mode = Some(FileActionDialogMode::SaveAs),
                Action::Revert => open_dialog_with_mode = Some(FileActionDialogMode::Revert),
                Action::Rename => open_dialog_with_mode = Some(FileActionDialogMode::Rename),
                Action::Delete => open_dialog_with_mode = Some(FileActionDialogMode::Delete),
                _ => {}
            };

            if let Some(mode) = open_dialog_with_mode {
                open_file_action_dialog(state, dialogs, dialog_state, mode, file_index);
            }
        }
    });
}

pub fn file_action_dialog(
    dialog_state: &mut FileActionDialogState,
    ctx: &mut Context,
) -> (bool, bool) {
    const WIDTH: f32 = 200.0;
    let input_name_with_ext = dialog_state.input_name.clone() + &dialog_state.file_extension;
    let overwrite_warning = dialog_state
        .all_other_file_names
        .contains(&input_name_with_ext);
    let overwrite_color = Color32::from_rgb(160, 70, 70);

    let mut close_dialog = false;
    let mut execute_action = false;

    fn filename_edit_field(ui: &mut Ui, value: &mut String) -> Response {
        const ALLOWED_CHARS: &str = "-_0123456789abcdefghijklmnopqrstuvwxyz";
        let res = ui.add_sized(
            [WIDTH, 20.0],
            TextEdit::singleline(value).char_limit(24).clip_text(true),
        );
        *value = value
            .chars()
            .map(|c| {
                if c == '-' {
                    '_'
                } else {
                    c.to_ascii_lowercase()
                }
            })
            .filter(|c| ALLOWED_CHARS.contains(c.to_owned()))
            .collect::<String>();
        res
    }

    Area::new(Id::new("file_action_dialog"))
        .default_width(WIDTH)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            Frame::none()
                .inner_margin(Margin::same(16.0))
                .rounding(Rounding::same(8.0))
                .stroke(if overwrite_warning {
                    Stroke::new(3.0, overwrite_color)
                } else {
                    Stroke::NONE
                })
                .fill(ui.style().visuals.panel_fill)
                .show(ui, |ui| {
                    ui.style_mut().spacing.item_spacing.y = 12.0;

                    ui.add(
                        Label::new(
                            RichText::new(dialog_state.mode.get_str("title").unwrap()).heading(),
                        )
                        .selectable(false),
                    );
                    ui.add(
                        Label::new(format!("File:       {}", dialog_state.current_name))
                            .selectable(false),
                    );

                    if dialog_state.mode == FileActionDialogMode::Revert {
                        ui.add(
                            Label::new("Are you sure you want to revert this file?")
                                .selectable(false),
                        );
                    } else if dialog_state.mode == FileActionDialogMode::Delete {
		        ui.add(
                            Label::new("Are you sure you want to delete this file?")
                                .selectable(false),
                        );
		    } else  {
                        ui.horizontal(|ui| {
                            ui.set_max_width(200.0);
                            ui.add(Label::new("Name:").selectable(false));
                            filename_edit_field(ui, &mut dialog_state.input_name);
                        });
                    }

                    if overwrite_warning {
                        ui.add(
                            Label::new(RichText::new("A file with this name already exists.\nDouble click \"Overwrite\" to overwrite it.").color(overwrite_color))
                                .selectable(false),
                        );
                    }

                    let (confirm_text, confirm_color) = if overwrite_warning {
                        ("Overwrite", overwrite_color)
                    } else {
                        (dialog_state.mode.get_str("confirm").unwrap(), Color32::from_rgb(45, 100, 45))
                    };

                    ui.with_layout(Layout::right_to_left(Align::Min), |ui| {
                        let confirm_button =
                            ui.add(Button::new(confirm_text).fill(confirm_color));
                        if (!overwrite_warning && confirm_button.clicked()) || confirm_button.double_clicked() {
                            execute_action = true;
                            close_dialog = true;
                            return;
                        };

                        if ui.add(Button::new("Cancel")).clicked() {
                            close_dialog = true;
                            return;
                        };
                    });
                });
        });

    return (close_dialog, execute_action);
}

pub fn execute_file_action_dialog_action(
    commands: &mut Commands,
    state: &mut EditorState,
    FileActionDialogState {
        mode,
        file_index,
        input_name,
        ..
    }: &mut FileActionDialogState,
) {
    // TODO handle errors
    match *mode {
        FileActionDialogMode::SaveAs => {
            state
                .files
                .save_file_with_name(*file_index, input_name.clone())
                .unwrap();
        }
        FileActionDialogMode::Rename => {
            state
                .files
                .rename_file(*file_index, input_name.clone())
                .unwrap();
        }
        FileActionDialogMode::Revert => {
            state.files.revert_file(*file_index).unwrap();
            commands.queue(RevertCommand);
        }
        FileActionDialogMode::Delete => {
            state.files.delete_file(*file_index).unwrap();
        }
    }

    input_name.clear();
}
