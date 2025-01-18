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
use consts::*;
use controls::{PlayerControlsPlugin, PlayerMotionConfig};

mod camera;
mod controls;

pub use camera::ForwardFromCamera;

mod consts {
    use avian3d::prelude::ColliderConstructor;

    pub const PLAYER_FLOAT_HEIGHT_FROM_GROUND: f32 = 0.5;
    pub const PLAYER_HEIGHT: f32 = 1.8288; // 6'
    pub const PLAYER_RADIUS: f32 = 0.25;
    pub const PLAYER_COLLIDER_HEIGHT: f32 =
        PLAYER_HEIGHT - PLAYER_RADIUS * 2.0 - PLAYER_FLOAT_HEIGHT_FROM_GROUND;
    pub const PLAYER_COLLIDER: ColliderConstructor = ColliderConstructor::Capsule {
        radius: PLAYER_RADIUS,
        height: PLAYER_COLLIDER_HEIGHT,
    };
    pub const PLAYER_FLOAT_HEIGHT_FROM_CENTER: f32 =
        PLAYER_FLOAT_HEIGHT_FROM_GROUND + PLAYER_HEIGHT / 2.0;

    pub const PLAYER_EYES_TO_CROWN_HEIGHT: f32 = 0.1524; // 6"
    pub const PLAYER_CENTER_TO_EYES_HEIGHT: f32 =
        PLAYER_COLLIDER_HEIGHT / 2.0 - PLAYER_EYES_TO_CROWN_HEIGHT;
}

#[derive(Component)]
pub struct IsPlayer;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
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
    cmd.insert(LockedAxes::new().lock_rotation_x().lock_rotation_z());
    cmd.insert(PLAYER_COLLIDER);
    cmd.insert(Sleeping);
    cmd.insert(TnuaToggle::Disabled);
    cmd.insert(GravityScale(0.0));

    cmd.insert(TnuaController::default());

    cmd.insert(PlayerMotionConfig {
        speed: 6.0,
        sprint_speed_multiplier: 1.75,
        crouch_speed_multiplier: 0.75,
        walk: TnuaBuiltinWalk {
            float_height: PLAYER_FLOAT_HEIGHT_FROM_CENTER,
            max_slope: FRAC_PI_4,
            turning_angvel: Float::INFINITY,
            ..Default::default()
        },
        jump: TnuaBuiltinJump {
            height: 2.25,
            shorten_extra_gravity: 0.0, // Disable variable height jumps
            ..Default::default()
        },
        crouch: TnuaBuiltinCrouch {
            float_offset: -0.7,
            height_change_impulse_limit: 5.0,
            ..Default::default()
        },
        actions_in_air: 0,
    });

    cmd.insert(ForwardFromCamera::default());

    cmd.insert(TnuaCrouchEnforcer::new(0.5 * Vector3::Y, |cmd| {
        let bundle = TnuaAvian3dSensorShape(
            Collider::try_from_constructor(PLAYER_COLLIDER, None)
                .expect("failed to create crouch enforcer collider"),
        );
        cmd.insert(bundle);
    }));

    cmd.insert(TnuaSimpleAirActionsCounter::default());
}
