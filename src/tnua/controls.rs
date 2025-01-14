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
    pub walk: TnuaBuiltinWalk,
    pub actions_in_air: usize,
    pub jump: TnuaBuiltinJump,
    pub crouch: TnuaBuiltinCrouch,
    pub dash_distance: f32,
    pub dash: TnuaBuiltinDash,
    pub knockback: TnuaBuiltinKnockback,
}

#[allow(clippy::type_complexity)]
#[allow(clippy::useless_conversion)]
pub fn apply_platformer_controls(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<(
        &PlayerMotionConfig,
        &mut TnuaController,
        &mut TnuaCrouchEnforcer,
        //&mut TnuaProximitySensor,
        &mut TnuaSimpleAirActionsCounter,
        Option<&ForwardFromCamera>,
    )>,
) {
    for (
        config,
        mut controller,
        mut crouch_enforcer,
        //mut sensor,
        mut air_actions_counter,
        forward_from_camera,
    ) in query.iter_mut()
    {
        // This part is just keyboard input processing. In a real game this would probably be done
        // with a third party plugin.
        let mut direction = Vector3::ZERO;

        // if config.dimensionality == Dimensionality::Dim3 {
        if keyboard.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
            direction -= Vector3::Z;
        }
        if keyboard.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
            direction += Vector3::Z;
        }
        // }
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
        let dash = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

        let turn_in_place = forward_from_camera.is_none()
            && keyboard.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]);

        let crouch_buttons = [KeyCode::ControlLeft, KeyCode::ControlRight];
        let crouch_pressed = keyboard.any_pressed(crouch_buttons);
        //let crouch_just_pressed = keyboard.any_just_pressed(crouch_buttons);
        let crouch = crouch_pressed;

        air_actions_counter.update(controller.as_mut());

        let speed_factor =
            if let Some((_, state)) = controller.concrete_action::<TnuaBuiltinCrouch>() {
                if matches!(state, TnuaBuiltinCrouchState::Rising) {
                    1.0
                } else {
                    0.2
                }
            } else {
                1.0
            };

        controller.basis(TnuaBuiltinWalk {
            desired_velocity: if turn_in_place {
                Vector3::ZERO
            } else {
                direction * speed_factor * config.speed
            },
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

        if dash {
            controller.action(TnuaBuiltinDash {
                displacement: direction.normalize() * config.dash_distance,
                desired_forward: if forward_from_camera.is_none() {
                    Dir3::new(direction).ok()
                } else {
                    None
                },
                allow_in_air: air_actions_counter.air_count_for(TnuaBuiltinDash::NAME)
                    <= config.actions_in_air,
                ..config.dash.clone()
            });
        }
    }
}
