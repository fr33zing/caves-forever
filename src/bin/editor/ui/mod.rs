use anyhow::anyhow;
use bevy::{
    app::{App, Plugin, Update},
    prelude::{MouseButton, ResMut, Resource, Single, With},
    window::{PrimaryWindow, Window},
};
use bevy_egui::{
    egui::{self, menu, Color32, Margin, Ui},
    EguiContexts,
};
use bevy_trackball::{TrackballCamera, TrackballController};
use egui::{
    vec2, Align2, Area, Frame, Id, Label, Layout, RichText, SelectableLabel, SidePanel,
    TopBottomPanel, Vec2, Visuals,
};
use nalgebra::{Point3, Vector3};
use strum::{EnumProperty, IntoEnumIterator};

use crate::{
    mode::tunnels,
    state::{EditorMode, EditorState, EditorViewMode},
};

mod file_browser;
mod icons;

use file_browser::{file_browser, save_as_dialog};

#[derive(Resource, Default)]
pub struct EditorDialogs {
    pub show_save_as_dialog: bool,
}

#[derive(Resource, Default)]
pub struct SaveAsDialogState {
    pub filename: String,
}

#[derive(Resource)]
pub struct SidePanelVisibility {
    pub left: bool,
    pub right: bool,
}

impl Default for SidePanelVisibility {
    fn default() -> Self {
        Self {
            left: true,
            right: false,
        }
    }
}

#[derive(Resource, Default)]
pub struct CursorOverEditSelectionPanel(pub bool);

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditorDialogs>();
        app.init_resource::<SidePanelVisibility>();
        app.init_resource::<SaveAsDialogState>();
        app.init_resource::<CursorOverEditSelectionPanel>();
        app.add_systems(Update, ui);
    }
}

fn ui(
    mut state: ResMut<EditorState>,
    mut side_panel_visibility: ResMut<SidePanelVisibility>,
    mut dialogs: ResMut<EditorDialogs>,
    mut save_as_dialog_state: ResMut<SaveAsDialogState>,
    mut cursor_over_edit_selection_panel: ResMut<CursorOverEditSelectionPanel>,
    mut contexts: EguiContexts,
    trackball: Option<Single<(&mut TrackballController, &mut TrackballCamera)>>,
    window: Option<Single<&Window, With<PrimaryWindow>>>,
) {
    let ctx = contexts.ctx_mut();
    ctx.set_visuals(Visuals::dark());

    // Top panel
    let top_panel_height = 30.0;
    let mut top_frame = Frame::side_top_panel(&ctx.style());
    top_frame.inner_margin = Margin::same(8.0);
    TopBottomPanel::top("top_panel")
        .frame(top_frame)
        .default_height(top_panel_height)
        .resizable(false)
        .show(ctx, |ui| {
            top_panel(&mut state, &mut dialogs, ui, trackball);
        });

    // Left panel
    let left_panel_width = if side_panel_visibility.left {
        200.0
    } else {
        0.0
    };
    if side_panel_visibility.left {
        let mut left_frame = Frame::side_top_panel(&ctx.style());
        left_frame.inner_margin = Margin::ZERO;
        SidePanel::left("file_browser")
            .frame(left_frame)
            .default_width(left_panel_width)
            .resizable(false)
            .show(ctx, |ui| {
                file_browser(&mut state, ui);
                ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
            });
    }

    // Right panel
    let right_panel_width = if side_panel_visibility.right {
        200.0
    } else {
        0.0
    };
    if side_panel_visibility.right {
        let mut right_frame = Frame::side_top_panel(&ctx.style());
        right_frame.inner_margin = Margin::same(8.0);
        SidePanel::right("selection_editor")
            .frame(right_frame)
            .default_width(right_panel_width)
            .resizable(false)
            .show(ctx, |ui| {
                match state.mode {
                    EditorMode::Tunnels => tunnels::sidebar(&mut state, ui),
                    EditorMode::Rooms => {}
                }
                ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
            });
    }
    cursor_over_edit_selection_panel.0 = if let Some(window) = window {
        if let Some(cursor) = window.cursor_position() {
            side_panel_visibility.right
                && cursor.x >= window.width() - right_panel_width
                && cursor.y > top_panel_height
        } else {
            false
        }
    } else {
        false
    };

    // Panel toggles
    Area::new(Id::new("toggle_left_panel"))
        .anchor(Align2::LEFT_BOTTOM, vec2(left_panel_width + 8.0, -8.0))
        .show(ctx, |ui| {
            ui.checkbox(&mut side_panel_visibility.left, "File browser");
        });
    let right_panel_toggle_hovered = Area::new(Id::new("toggle_right_panel"))
        .anchor(Align2::RIGHT_BOTTOM, vec2(-right_panel_width - 8.0, -8.0))
        .show(ctx, |ui| {
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut side_panel_visibility.right, "Properties")
            })
        })
        .inner
        .inner
        .contains_pointer();
    cursor_over_edit_selection_panel.0 =
        cursor_over_edit_selection_panel.0 || right_panel_toggle_hovered;

    // Save as dialog
    let save_as_result = egui::Window::new("Save as...")
        .default_width(200.0)
        .max_width(200.0)
        .resizable(false)
        .collapsible(false)
        .open(&mut dialogs.show_save_as_dialog)
        .show(ctx, |ui| {
            save_as_dialog(&mut state, &mut save_as_dialog_state, ui)
        });
    if let Some(inner) = save_as_result {
        if let Some((close, save)) = inner.inner {
            if close {
                dialogs.show_save_as_dialog = false;
            }
            if save {
                save_current(&mut state, &mut dialogs).unwrap();
            }
        }
    }
}

fn top_panel(
    state: &mut EditorState,
    dialogs: &mut EditorDialogs,
    ui: &mut Ui,
    trackball: Option<Single<(&mut TrackballController, &mut TrackballCamera)>>,
) {
    ui.horizontal(|ui| {
        // Menu bar
        Frame::none().show(ui, |ui| {
            ui.shrink_width_to_current();
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    file_menu(state, dialogs, ui);
                });
                ui.menu_button("Viewport", |ui| {
                    let allow_orbit = !(state.mode == EditorMode::Tunnels
                        && state.view == EditorViewMode::Editor);
                    viewport_menu(ui, allow_orbit, trackball);
                });
            });
        });

        ui.separator();

        if let Some(picker) = state.file_picker() {
            let current = picker.current_file();
            if let Some(current) = current {
                ui.add(Label::new(current.name.clone()).selectable(false));
                if current.changed {
                    icons::changed_default(ui);
                }

                ui.separator();
            }
        }

        // Mode switcher
        ui.label("Mode:");
        EditorMode::iter().for_each(|mode| {
            let button = ui.add_enabled(
                state.mode != mode,
                egui::Button::new(mode.get_str("Name").unwrap()),
            );
            if button.clicked() {
                state.mode = mode;
            }
        });

        ui.separator();

        // View switcher
        ui.label("View:");
        EditorViewMode::iter().for_each(|mode| {
            let button = ui.add_enabled(
                state.view != mode,
                egui::Button::new(mode.get_str("Name").unwrap()),
            );
            if button.clicked() {
                state.view = mode;
            }
        });

        ui.separator();

        match state.mode {
            EditorMode::Tunnels => match state.view {
                EditorViewMode::Editor => {
                    ui.checkbox(&mut state.tunnels_mode.mirror, "Mirror");
                }
                EditorViewMode::Preview => {}
            },
            EditorMode::Rooms => match state.view {
                EditorViewMode::Editor => {}
                EditorViewMode::Preview => {}
            },
        }
    });
}

fn file_menu(state: &mut EditorState, dialogs: &mut EditorDialogs, ui: &mut Ui) {
    if ui.selectable_label(false, "New").clicked() {};
    if ui.selectable_label(false, "Duplicate").clicked() {};
    if ui.selectable_label(false, "Delete").clicked() {};

    ui.separator();

    if ui.selectable_label(false, "Save").clicked() {
        ui.close_menu();
        // TODO handle this
        save_current(state, dialogs).expect("save failed");
    };
    if ui.selectable_label(false, "Save as...").clicked() {
        ui.close_menu();
        save_as(dialogs);
    };
    if ui.selectable_label(false, "Save all").clicked() {};
}

fn viewport_menu(
    ui: &mut Ui,
    allow_orbit: bool,
    trackball: Option<Single<(&mut TrackballController, &mut TrackballCamera)>>,
) {
    let Some(trackball) = trackball else {
        return;
    };

    let (mut controller, mut camera) = trackball.into_inner();

    if ui.selectable_label(false, "Reset").clicked() {
        camera.frame = camera.reset;
        ui.close_menu();
    };

    ui.add_enabled_ui(allow_orbit, |ui| {
        ui.menu_button("Align", |ui| {
            let (d, target, up, eps) = (16.0, Point3::origin(), &Vector3::y_axis(), f32::EPSILON);
            let mut options = Vec::<(RichText, Point3<f32>)>::new();
            let (red, green, blue) = (
                Color32::from_rgb(255, 100, 100),
                Color32::from_rgb(100, 230, 100),
                Color32::from_rgb(100, 100, 255),
            );
            options.push((RichText::new("-X").color(red), Point3::new(d, 0.0, 0.0)));
            options.push((RichText::new("+X").color(red), Point3::new(-d, 0.0, 0.0)));
            options.push((RichText::new("-Y").color(green), Point3::new(0.0, d, eps)));
            options.push((RichText::new("+Y").color(green), Point3::new(0.0, -d, eps)));
            options.push((RichText::new("-Z").color(blue), Point3::new(0.0, 0.0, d)));
            options.push((RichText::new("+Z").color(blue), Point3::new(0.0, 0.0, -d)));

            options.chunks(2).for_each(|options| {
                ui.horizontal(|ui| {
                    for (direction, eye) in options.into_iter() {
                        if ui
                            .add_sized([20.0, 20.0], SelectableLabel::new(false, direction.clone()))
                            .clicked()
                        {
                            camera.scope.set_ortho(true);
                            camera.frame.set_target(target);
                            camera.frame.set_eye(&eye, up);
                            ui.close_menu();
                        };
                    }
                });
            });
        });
    });

    ui.separator();

    ui.add_enabled_ui(allow_orbit, |ui| {
        let ortho = camera.scope.ortho();
        if ui.radio(ortho, "Orthographic").clicked() {
            camera.scope.set_ortho(true);
            ui.close_menu();
        }
        if ui.radio(!ortho, "Perspective").clicked() {
            camera.scope.set_ortho(false);
            ui.close_menu();
        }
    });

    ui.separator();

    let mut swapped = controller.input.slide_button != Some(MouseButton::Right);
    if ui.checkbox(&mut swapped, "Swap orbit/pan").clicked() {
        let (mut orbit, slide) = if swapped {
            (Some(MouseButton::Right), Some(MouseButton::Middle))
        } else {
            (Some(MouseButton::Middle), Some(MouseButton::Right))
        };
        if !allow_orbit {
            orbit = None;
        }
        controller.input.orbit_button = orbit;
        controller.input.slide_button = slide;
        ui.close_menu();
    }
}

//
// Utility
//

/// Save the current file OR open the "save as" dialog if it has no path
pub fn save_current(state: &mut EditorState, dialogs: &mut EditorDialogs) -> anyhow::Result<()> {
    let Some(picker) = state.file_picker_mut() else {
        return Err(anyhow!("current mode has no file picker"));
    };
    let path = match picker.current {
        Some(ref path) => path.clone(),
        None => {
            save_as(dialogs);
            return Ok(());
        }
    };
    let Some(file) = picker.current_file_mut() else {
        return Err(anyhow!("no current file"));
    };

    file.write(path.to_path_buf())?;

    Ok(())
}

// Open the "save as" dialog
pub fn save_as(dialogs: &mut EditorDialogs) {
    dialogs.show_save_as_dialog = true;
}
