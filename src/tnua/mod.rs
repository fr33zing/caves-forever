use std::f32::consts::FRAC_PI_4;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_tnua::{
    builtins::TnuaBuiltinCrouch,
    control_helpers::{TnuaCrouchEnforcer, TnuaCrouchEnforcerPlugin, TnuaSimpleAirActionsCounter},
    math::{Float, Vector3},
    prelude::{TnuaBuiltinJump, TnuaBuiltinWalk, TnuaController, TnuaControllerPlugin},
    TnuaToggle,
};
use bevy_tnua_avian3d::{TnuaAvian3dPlugin, TnuaAvian3dSensorShape};
use camera::PlayerCameraPlugin;
use controls::{PlayerControlsPlugin, PlayerMotionConfig};

mod camera;
mod controls;

pub use camera::ForwardFromCamera;

#[derive(Component)]
pub struct IsPlayer;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        // app.insert_resource(Time::from_hz(144.0));
        app.add_plugins((
            TnuaAvian3dPlugin::new(PhysicsSchedule),
            TnuaControllerPlugin::new(PhysicsSchedule),
            TnuaCrouchEnforcerPlugin::new(PhysicsSchedule),
            PlayerCameraPlugin,
            PlayerControlsPlugin,
        ));

        app.add_systems(Startup, setup_player);
    }
}

fn setup_player(mut commands: Commands) {
    let mut cmd = commands.spawn(IsPlayer);

    cmd.insert(Transform::from_translation(Vec3::new(32.0, 32.0, -32.0)));

    cmd.insert(RigidBody::Dynamic);
    cmd.insert(Collider::capsule(0.5, 1.0));

    cmd.insert(TnuaController::default());

    cmd.insert(PlayerMotionConfig {
        speed: 20.0,
        walk: TnuaBuiltinWalk {
            float_height: 2.0,
            max_slope: FRAC_PI_4,
            turning_angvel: Float::INFINITY,
            ..Default::default()
        },
        actions_in_air: 0, // Disable double jump
        jump: TnuaBuiltinJump {
            height: 2.0,
            shorten_extra_gravity: 0.0, // Disable variable height jumps
            ..Default::default()
        },
        crouch: TnuaBuiltinCrouch {
            float_offset: -0.9,
            ..Default::default()
        },
        dash_distance: 10.0,
        dash: Default::default(),
        knockback: Default::default(),
    });

    cmd.insert(ForwardFromCamera::default());

    cmd.insert(TnuaToggle::default());

    cmd.insert(TnuaCrouchEnforcer::new(0.5 * Vector3::Y, |cmd| {
        let bundle = TnuaAvian3dSensorShape(Collider::cylinder(0.5, 0.0));
        cmd.insert(bundle);
    }));

    cmd.insert(TnuaSimpleAirActionsCounter::default());
}
