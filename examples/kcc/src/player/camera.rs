use std::f32::consts::FRAC_PI_2;

use avian3d::prelude::*;
use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    render::view::RenderLayers,
    window::{CursorGrabMode, PrimaryWindow},
};
use lib::render_layer;

use super::{
    config::{PlayerCameraConfig, PlayerCameraMode},
    Player, PlayerConfig, PlayerKeybinds, Section,
};

const MOUSE_MOTION_SCALE: f32 = 0.00015;
const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.01;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Component)]
pub struct PlayerCameraChild;

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                add_required_components,
                toggle_cursor_lock,
                mouselook,
                (switch_camera_mode, transition_camera_mode).chain(),
            ),
        );
        app.add_systems(
            PostUpdate,
            attach_to_player
                .after(PhysicsSet::Sync)
                .before(TransformSystem::TransformPropagate),
        );
    }
}

fn add_required_components(
    mut commands: Commands,
    player_config: Res<PlayerConfig>,
    camera_config: Res<PlayerCameraConfig>,
    player_cameras: Query<(Entity, Option<&Children>), Added<PlayerCamera>>,
    cameras: Query<Entity, With<PlayerCameraChild>>,
) {
    player_cameras.iter().for_each(|(parent, children)| {
        commands
            .entity(parent)
            .insert_if_new(Visibility::Visible)
            .insert_if_new(Transform::from_translation(Vec3::new(
                0.0,
                player_config.height - camera_config.eye_offset,
                0.0,
            )));

        let child = 'find_child: {
            let Some(children) = children else {
                break 'find_child None;
            };
            for child in children {
                if let Ok(child) = cameras.get(*child) {
                    break 'find_child Some(child);
                }
            }
            None
        };

        let child = if let Some(child) = child {
            child
        } else {
            let child = commands.spawn(PlayerCameraChild).id();
            commands.entity(parent).add_child(child);
            child
        };

        let mut commands = commands.entity(child);
        commands
            .insert_if_new(RenderLayers::layer(render_layer::WORLD))
            .insert_if_new(Transform::default())
            .insert_if_new(Camera3d::default())
            .insert_if_new(Projection::Perspective(PerspectiveProjection {
                fov: camera_config.fov_degrees.to_radians(),
                ..default()
            }))
            .insert_if_new(SpatialListener::new(-player_config.radius * 2.0));
    });
}

fn toggle_cursor_lock(
    keyboard: Res<ButtonInput<KeyCode>>,
    window: Option<Single<&mut Window, With<PrimaryWindow>>>,
) {
    let Some(mut window) = window else {
        return;
    };

    if !keyboard.just_pressed(KeyCode::Escape) {
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

fn mouselook(
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    config: Res<PlayerCameraConfig>,
    mouse: Res<AccumulatedMouseMotion>,
    camera: Option<Single<&mut Transform, With<PlayerCamera>>>,
) {
    let Some(window) = window else {
        return;
    };
    if window.cursor_options.visible {
        return;
    }
    let Some(mut camera) = camera else {
        return;
    };
    if mouse.delta.length() == 0.0 {
        return;
    }

    let window_scale = {
        let Vec2 { x: w, y: h } = window.size();
        if w < h {
            Vec2::new(w / h, 1.0)
        } else {
            Vec2::new(1.0, h / w)
        }
    };

    let delta = mouse.delta * window_scale * config.sensitivity * MOUSE_MOTION_SCALE;
    let (yaw, pitch, _) = camera.rotation.to_euler(EulerRot::YXZ);
    let pitch = (pitch - delta.y).clamp(-PITCH_LIMIT, PITCH_LIMIT);
    let yaw = yaw - delta.x;
    camera.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
}

fn switch_camera_mode(
    keybinds: Res<PlayerKeybinds>,
    mut config: ResMut<PlayerCameraConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    let Some(ref switch_camera) = keybinds.switch_camera else {
        return;
    };

    if switch_camera.just_released(&keyboard, &mouse) {
        config.mode = match config.mode {
            PlayerCameraMode::FirstPerson => PlayerCameraMode::ThirdPerson,
            PlayerCameraMode::ThirdPerson => PlayerCameraMode::FirstPerson,
        }
    }
}

fn transition_camera_mode(
    time: Res<Time>,
    config: Res<PlayerCameraConfig>,
    mut camera_children: Query<&mut Transform, With<PlayerCameraChild>>,
) {
    camera_children.iter_mut().for_each(|mut child| {
        let target = match config.mode {
            PlayerCameraMode::FirstPerson => 0.0,
            PlayerCameraMode::ThirdPerson => config.third_person_distance,
        };
        let fac = (10.0 * time.delta_secs()).min(1.0);

        child.translation.z = child.translation.z.lerp(target, fac);
    });
}

fn attach_to_player(
    config: Res<PlayerCameraConfig>,
    camera: Option<Single<&mut Transform, With<PlayerCamera>>>,
    player: Option<Single<(&GlobalTransform, &Section), With<Player>>>,
) {
    let Some(player) = player else {
        return;
    };
    let Some(mut camera) = camera else {
        return;
    };

    let (player, section) = player.into_inner();

    camera.translation =
        player.translation() + Vec3::Y * (section.height + section.offset - config.eye_offset);
}
