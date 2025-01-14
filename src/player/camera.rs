use avian3d::prelude::*;
use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};

use super::motion::KinematicCharacterController;

const SENSITIVITY: f32 = 0.00000275;

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup)
            .add_systems(PostUpdate, update.after(PhysicsSet::Sync))
            .add_systems(Update, grab_cursor);
    }
}

#[derive(Component)]
pub struct PlayerCamera;

fn setup(mut commands: Commands) {
    commands.spawn((
        PlayerCamera,
        Camera3d { ..default() },
        SpotLight {
            intensity: 8_000_000.0,
            color: Color::WHITE.into(),
            shadows_enabled: true,
            inner_angle: 0.1,
            outer_angle: 0.3,
            range: 4000.0,
            radius: 4000.0,
            ..default()
        },
        Transform::from_translation(Vec3::new(32.0, 32.0, -32.0)),
    ));
}

fn toggle_grab_cursor(window: &mut Window) {
    match window.cursor_options.grab_mode {
        CursorGrabMode::None => {
            window.cursor_options.grab_mode = CursorGrabMode::Confined;
            window.cursor_options.visible = false;
        }
        _ => {
            window.cursor_options.grab_mode = CursorGrabMode::None;
            window.cursor_options.visible = true;
        }
    }
}

fn update(
    primary_window: Query<&Window, With<PrimaryWindow>>,
    mut state: EventReader<MouseMotion>,
    player_query: Query<&mut Position, With<KinematicCharacterController>>,
    mut camera_query: Query<&mut Transform, With<PlayerCamera>>,
) {
    for mut transform in camera_query.iter_mut() {
        if let Ok(player_position) = player_query.get_single() {
            transform.translation = **player_position + Vec3::new(0.0, 1.4, 0.0);
        }
    }

    if let Ok(window) = primary_window.get_single() {
        for mut transform in camera_query.iter_mut() {
            for ev in state.read() {
                let (mut yaw, mut pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
                match window.cursor_options.grab_mode {
                    CursorGrabMode::None => (),
                    _ => {
                        // Using smallest of height or width ensures equal vertical and horizontal sensitivity
                        let window_scale = window.height().min(window.width());
                        pitch -= (SENSITIVITY * ev.delta.y * window_scale).to_radians();
                        yaw -= (SENSITIVITY * ev.delta.x * window_scale).to_radians();
                    }
                }

                pitch = pitch.clamp(-1.54, 1.54);

                // Order is important to prevent unintended roll
                transform.rotation =
                    Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch);
            }
        }
    } else {
        warn!("Primary window not found for `player_look`!");
    }
}

fn grab_cursor(
    keys: Res<ButtonInput<KeyCode>>,
    mut primary_window: Query<&mut Window, With<PrimaryWindow>>,
) {
    if let Ok(mut window) = primary_window.get_single_mut() {
        if keys.just_pressed(KeyCode::Escape) {
            toggle_grab_cursor(&mut window);
        }
    } else {
        warn!("Primary window not found for `cursor_grab`!");
    }
}
