use avian3d::prelude::PhysicsSchedule;
use bevy::prelude::*;
use bevy_tnua::{
    builtins::{TnuaBuiltinCrouch, TnuaBuiltinCrouchState, TnuaBuiltinDash, TnuaBuiltinKnockback},
    control_helpers::{TnuaCrouchEnforcer, TnuaSimpleAirActionsCounter},
    math::Vector3,
    prelude::{TnuaBuiltinJump, TnuaBuiltinWalk, TnuaController},
    TnuaAction, TnuaUserControlsSystemSet,
};

use super::camera::ForwardFromCamera;

pub struct PlayerControlsPlugin;

impl Plugin for PlayerControlsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PhysicsSchedule,
            apply_platformer_controls.in_set(TnuaUserControlsSystemSet),
        );
    }
}

#[derive(Component)]
pub struct PlayerMotionConfig {
    pub speed: f32,
    pub sprint_speed_multiplier: f32,
    pub crouch_speed_multiplier: f32,
    pub walk: TnuaBuiltinWalk,
    pub jump: TnuaBuiltinJump,
    pub crouch: TnuaBuiltinCrouch,
    pub actions_in_air: usize,
}

#[allow(clippy::type_complexity)]
#[allow(clippy::useless_conversion)]
pub fn apply_platformer_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(
        &PlayerMotionConfig,
        &mut TnuaController,
        &mut TnuaCrouchEnforcer,
        &mut TnuaSimpleAirActionsCounter,
        Option<&ForwardFromCamera>,
    )>,
) {
    for (
        config,
        mut controller,
        mut crouch_enforcer,
        mut air_actions_counter,
        forward_from_camera,
    ) in query.iter_mut()
    {
        let mut direction = Vector3::ZERO;

        if keyboard.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
            direction -= Vector3::Z;
        }
        if keyboard.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
            direction += Vector3::Z;
        }
        if keyboard.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
            direction -= Vector3::X;
        }
        if keyboard.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
            direction += Vector3::X;
        }

        direction = direction.clamp_length_max(1.0);

        if let Some(forward_from_camera) = forward_from_camera {
            direction = Transform::default()
                .looking_to(forward_from_camera.forward, Vec3::Y)
                .transform_point(direction)
        }

        let jump = keyboard.any_pressed([KeyCode::Space]);
        let sprint = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
        let crouch = keyboard.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);

        air_actions_counter.update(controller.as_mut());

        let speed_factor =
            if let Some((_, state)) = controller.concrete_action::<TnuaBuiltinCrouch>() {
                if matches!(state, TnuaBuiltinCrouchState::Rising) {
                    1.0
                } else {
                    0.2
                }
            } else if sprint {
                config.sprint_speed_multiplier
            } else {
                1.0
            };

        controller.basis(TnuaBuiltinWalk {
            desired_velocity: direction * speed_factor * config.speed,
            desired_forward: Dir3::new(forward_from_camera.unwrap().forward).ok(),
            ..config.walk.clone()
        });

        if crouch {
            controller.action(crouch_enforcer.enforcing(config.crouch.clone()));
        }

        if jump {
            controller.action(TnuaBuiltinJump {
                allow_in_air: air_actions_counter.air_count_for(TnuaBuiltinJump::NAME)
                    <= config.actions_in_air,
                ..config.jump.clone()
            });
        }
    }
}
