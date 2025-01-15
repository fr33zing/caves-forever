use std::f32::consts::FRAC_PI_2;

use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_egui::{egui, EguiContexts};
use bevy_tnua::math::{Float, Vector3};

use super::PLAYER_CENTER_TO_EYES_HEIGHT;

const MOUSE_MOTION_SCALE: f32 = 0.00015;

#[derive(Component)]
pub struct ForwardFromCamera {
    pub forward: Vector3,
    pub pitch_angle: Float,
}

impl Default for ForwardFromCamera {
    fn default() -> Self {
        Self {
            forward: Vector3::NEG_Z,
            pitch_angle: 0.0,
        }
    }
}

#[derive(Resource)]
struct UiState {
    sensitivity: f32,
}

impl Default for UiState {
    #[cfg(not(feature = "webgl2"))]
    fn default() -> Self {
        Self { sensitivity: 1.0 }
    }

    #[cfg(feature = "webgl2")]
    fn default() -> Self {
        Self { sensitivity: 5.0 }
    }
}

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>();
        app.add_systems(Update, ui);

        app.add_systems(Startup, setup);
        app.add_systems(Update, grab_ungrab_mouse);
        app.add_systems(PostUpdate, {
            apply_camera_controls.before(bevy::transform::TransformSystem::TransformPropagate)
        });
    }
}

fn float_edit_field(ui: &mut egui::Ui, value: &mut f32) -> egui::Response {
    let mut tmp_value = format!("{:.4}", value);
    let res = ui.text_edit_singleline(&mut tmp_value);
    if let Ok(result) = tmp_value.parse() {
        *value = result;
    }
    res
}

fn ui(
    window: Single<&Window, With<PrimaryWindow>>,
    mut ui_state: ResMut<UiState>,
    mut contexts: EguiContexts,
) {
    if !window.cursor_options.visible {
        return;
    }

    let w = 256.0;
    egui::Window::new("Info")
        .fixed_pos(egui::pos2(window.width() / 2.0 - w / 2.0, 16.0))
        .default_width(w)
        .title_bar(false)
        .resizable(false)
        .show(contexts.ctx_mut(), |ui| {
            ui.label("Press T to toggle camera control.");
            ui.label("Left click to destroy terrain.");

            ui.add_space(10.0);

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label("Sensitivity: ");
                    float_edit_field(ui, &mut ui_state.sensitivity);
                });
            });
        });
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d { ..default() },
        Projection::Perspective(PerspectiveProjection {
            fov: 45.0_f32.to_radians(),
            ..default()
        }),
        SpotLight {
            intensity: 12_000_000.0,
            color: Color::WHITE.into(),
            shadows_enabled: true,
            inner_angle: 0.1,
            outer_angle: 0.5,
            range: 4000.0,
            radius: 4000.0,
            ..default()
        },
    ));
}

fn grab_ungrab_mouse(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyT) {
        return;
    }

    if window.cursor_options.visible
        || !matches!(window.cursor_options.grab_mode, CursorGrabMode::Locked)
    {
        window.cursor_options.grab_mode = CursorGrabMode::Locked;
        window.cursor_options.visible = false;
    } else {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}

fn apply_camera_controls(
    primary_window_query: Query<&Window, With<PrimaryWindow>>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut player_character_query: Query<(&GlobalTransform, &mut ForwardFromCamera)>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
    ui_state: Res<UiState>,
) {
    let mouse_controls_camera = primary_window_query
        .get_single()
        .map_or(false, |w| !w.cursor_options.visible);

    let total_delta = if mouse_controls_camera {
        mouse_motion.read().map(|event| event.delta).sum()
    } else {
        Vec2::ZERO
    };
    mouse_motion.clear();

    let window_scale = if let Ok(window) = primary_window_query.get_single() {
        let Vec2 { x: w, y: h } = window.size();

        if w < h {
            Vec2::new(w / h, 1.0)
        } else if w > h {
            Vec2::new(1.0, h / w)
        } else {
            Vec2::ONE
        }
    } else {
        Vec2::ONE
    };

    let total_delta = total_delta * MOUSE_MOTION_SCALE * ui_state.sensitivity * window_scale;

    let Ok((player_transform, mut forward_from_camera)) = player_character_query.get_single_mut()
    else {
        return;
    };

    let yaw = Quat::from_rotation_y(-total_delta.x);
    let pitch = total_delta.y;

    forward_from_camera.forward = yaw.mul_vec3(forward_from_camera.forward);
    forward_from_camera.pitch_angle =
        (forward_from_camera.pitch_angle + pitch).clamp(-FRAC_PI_2, FRAC_PI_2);

    for mut camera in camera_query.iter_mut() {
        camera.translation =
            player_transform.translation() + PLAYER_CENTER_TO_EYES_HEIGHT * Vec3::Y;
        //camera.translation -= 5.0 * forward_from_camera.forward; // 3rd person view
        camera.look_to(forward_from_camera.forward, Vec3::Y);
        let pitch_axis = camera.left();
        camera.rotate_around(
            player_transform.translation(),
            Quat::from_axis_angle(*pitch_axis, forward_from_camera.pitch_angle),
        );
    }
}
