use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_tnua::{control_helpers::TnuaCrouchEnforcerPlugin, prelude::TnuaControllerPlugin};
use bevy_tnua_avian3d::TnuaAvian3dPlugin;
use camera::PlayerCameraPlugin;
use consts::*;
use controls::PlayerControlsPlugin;

mod camera;
mod controls;
mod spawn;

pub use camera::ForwardFromCamera;
pub use spawn::*;

pub mod consts {
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
    }
}
