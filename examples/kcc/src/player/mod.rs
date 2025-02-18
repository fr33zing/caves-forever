use avian3d::prelude::{LockedAxes, RigidBody};
use bevy::{pbr::NotShadowCaster, prelude::*};

mod config;
mod motion;
mod quakeish;
mod utility;

#[cfg(any(feature = "first-person-camera", feature = "third-person-camera"))]
mod camera;
#[cfg(any(feature = "first-person-camera", feature = "third-person-camera"))]
pub use camera::PlayerCamera;
#[cfg(any(feature = "first-person-camera", feature = "third-person-camera"))]
use camera::PlayerCameraPlugin;

#[cfg(feature = "input")]
mod input;
#[cfg(feature = "input")]
use input::PlayerInputPlugin;

#[cfg(feature = "crouch")]
mod crouch;
#[cfg(feature = "crouch")]
use crouch::PlayerCrouchPlugin;

use config::PlayerCameraConfig;
pub use config::{PlayerConfig, PlayerKeybinds};
pub use motion::PlayerMotion;
use motion::PlayerMotionPlugin;
pub use utility::{Section, SectionShape};

#[derive(Component)]
pub struct Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerConfig>();
        app.init_resource::<PlayerKeybinds>();
        app.init_resource::<PlayerCameraConfig>();
        app.add_plugins((
            PlayerMotionPlugin,
            #[cfg(any(feature = "first-person-camera", feature = "third-person-camera"))]
            PlayerCameraPlugin,
            #[cfg(feature = "input")]
            PlayerInputPlugin,
            #[cfg(feature = "crouch")]
            PlayerCrouchPlugin,
        ));
        app.add_systems(Update, add_required_components);
    }
}

fn add_required_components(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    config: Res<PlayerConfig>,
    players: Query<Entity, Added<Player>>,
) {
    let section = Section {
        shape: SectionShape::Capsule,
        offset: 0.0,
        height: config.height,
        radius: config.radius,
    };

    players.iter().for_each(|parent| {
        commands
            .entity(parent)
            .insert(section.clone())
            .insert(PlayerMotion::default())
            .insert(LockedAxes::ROTATION_LOCKED)
            .insert(RigidBody::Kinematic)
            .insert_if_new(section.collider())
            .insert_if_new(Visibility::Visible)
            .insert_if_new(Transform::default())
            .insert_if_new(NotShadowCaster)
            .insert_if_new(Mesh3d(meshes.add(section.mesh())))
            .insert_if_new(MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.4, 1.0, 0.4),
                ..default()
            })));
    });
}
