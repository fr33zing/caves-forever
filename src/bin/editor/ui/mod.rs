mod icons;

use core::f32;
use std::collections::HashMap;

use bevy::{
    app::{App, Plugin, Update},
    math::Vec3,
    prelude::{MouseButton, ResMut, Single},
};
use bevy_egui::{
    egui::{self, menu, Color32, Margin, Ui},
    EguiContexts,
};
use bevy_trackball::{prelude::Frame, TrackballCamera, TrackballController};
use egui::{RichText, SelectableLabel};
use nalgebra::{Point3, Vector3};
use strum::{EnumProperty, IntoEnumIterator};

use crate::state::{EditorMode, EditorState, EditorViewMode};

pub struct EditorUiPlugin;

impl Plugin for EditorUiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, ui);
    }
}

fn ui(
    mut state: ResMut<EditorState>,
    mut contexts: EguiContexts,
    trackball: Option<Single<(&mut TrackballController, &mut TrackballCamera)>>,
) {
    let ctx = contexts.ctx_mut();

    ctx.style_mut(|style| {
        style.visuals.panel_fill = Color32::from_rgba_premultiplied(25, 25, 25, 235);
    });

    let mut frame = egui::Frame::side_top_panel(&ctx.style());
    frame.inner_margin = Margin {
        left: 8.0,
        right: 8.0,
        top: 8.0,
        bottom: 8.0,
    };

    // Left panel
    // SidePanel::left("left_panel")
    //     .frame(frame)
    //     .max_width(300.0)
    //     .show(ctx, |ui| {
    //         left_panel(&mut state, ui);
    //         ui.allocate_rect(ui.available_rect_before_wrap(), egui::Sense::hover());
    //     });

    // Top panel
    egui::TopBottomPanel::top("top_panel")
        .frame(frame)
        .show(ctx, |ui| {
            top_panel(&mut state, ui, trackball);
        });
}

fn top_panel(
    state: &mut EditorState,
    ui: &mut Ui,
    trackball: Option<Single<(&mut TrackballController, &mut TrackballCamera)>>,
) {
    ui.horizontal(|ui| {
        // Menu bar
        egui::Frame::none().show(ui, |ui| {
            ui.shrink_width_to_current();
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    file_menu(ui);
                });
                ui.menu_button("Viewport", |ui| {
                    let allow_orbit = !(state.mode == EditorMode::Tunnels
                        && state.view == EditorViewMode::Editor);
                    viewport_menu(ui, allow_orbit, trackball);
                });
            });
        });

        ui.separator();

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

fn file_menu(ui: &mut Ui) {
    if ui.selectable_label(false, "Open").clicked() {};

    ui.separator();

    if ui.selectable_label(false, "New").clicked() {};
    if ui.selectable_label(false, "Duplicate").clicked() {};
    if ui.selectable_label(false, "Delete").clicked() {};

    ui.separator();

    if ui.selectable_label(false, "Save").clicked() {};
    if ui.selectable_label(false, "Save as").clicked() {};
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
