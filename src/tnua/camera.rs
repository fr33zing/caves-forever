use std::f32::consts::FRAC_PI_2;

use bevy::{
    input::mouse::MouseMotion,
    prelude::*,
    window::{CursorGrabMode, PrimaryWindow},
};
use bevy_tnua::math::{Float, Quaternion, Vector3};

const SENSITIVITY: f32 = 0.01;

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

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Update, grab_ungrab_mouse);
        app.add_systems(PostUpdate, {
            apply_camera_controls.before(bevy::transform::TransformSystem::TransformPropagate)
        });
    }
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
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
) {
    if window.cursor_options.visible {
        if mouse_buttons.just_pressed(MouseButton::Left) {
            window.cursor_options.grab_mode = CursorGrabMode::Locked;
            window.cursor_options.visible = false;
        }
    } else if keyboard.just_released(KeyCode::Escape)
        || mouse_buttons.just_pressed(MouseButton::Left)
    {
        window.cursor_options.grab_mode = CursorGrabMode::None;
        window.cursor_options.visible = true;
    }
}

fn apply_camera_controls(
    mut mouse_motion: EventReader<MouseMotion>,
    mut camera_query: Query<&mut Transform, With<Camera>>,
    window: Option<Single<&mut Window, With<PrimaryWindow>>>,
    player: Option<Single<(&GlobalTransform, &mut ForwardFromCamera)>>,
) {
    let (Some(window), Some(player)) = (window, player) else {
        mouse_motion.clear();
        return;
    };

    let total_delta = if window.cursor_options.visible {
        Vec2::ZERO
    } else {
        mouse_motion.read().map(|event| event.delta).sum()
    };
    mouse_motion.clear();

    let total_delta = total_delta * SENSITIVITY;
    let (player_transform, mut forward_from_camera) = player.into_inner();
    let yaw = Quaternion::from_rotation_y(-0.01 * total_delta.x);
    let pitch = 0.005 * total_delta.y;

    forward_from_camera.forward = yaw.mul_vec3(forward_from_camera.forward);
    forward_from_camera.pitch_angle =
        (forward_from_camera.pitch_angle + pitch).clamp(-FRAC_PI_2, FRAC_PI_2);

    for mut camera in camera_query.iter_mut() {
        let pitch_axis = camera.left();

        camera.translation = player_transform.translation() + 1.0 * Vec3::Y;
        camera.look_to(forward_from_camera.forward, Vec3::Y);
        camera.rotate_around(
            player_transform.translation(),
            Quat::from_axis_angle(*pitch_axis, forward_from_camera.pitch_angle),
        );
    }
}
