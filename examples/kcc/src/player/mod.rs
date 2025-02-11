use avian3d::prelude::LockedAxes;
use bevy::prelude::*;

mod camera;
mod config;
mod motion;
mod utility;

use camera::PlayerCameraPlugin;
use config::PlayerCameraConfig;

pub use camera::PlayerCamera;
pub use config::{PlayerConfig, PlayerKeybinds};
use motion::{PlayerMotion, PlayerMotionPlugin};
pub use utility::{Section, SectionShape};

#[derive(Component)]
pub struct Player;

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerConfig>();
        app.init_resource::<PlayerKeybinds>();
        app.init_resource::<PlayerCameraConfig>();
        app.add_plugins((PlayerCameraPlugin, PlayerMotionPlugin));
        app.add_systems(Update, add_required_components);
    }
}

fn add_required_components(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    config: Res<PlayerConfig>,
    players: Query<(Entity, Option<&Children>), Added<Player>>,
    cameras: Query<Entity, With<PlayerCamera>>,
) {
    let section = Section {
        shape: SectionShape::Capsule,
        offset: 0.0,
        height: config.height,
        radius: config.radius,
    };

    players.iter().for_each(|(parent, children)| {
        println!("{}", "asdas");
        commands
            .entity(parent)
            .insert(section.clone())
            .insert(PlayerMotion::default())
            .insert(LockedAxes::ROTATION_LOCKED)
            .insert_if_new(section.collider())
            .insert_if_new(Visibility::Visible)
            .insert_if_new(Transform::default())
            .insert_if_new(Mesh3d(meshes.add(section.mesh())))
            .insert_if_new(MeshMaterial3d(materials.add(StandardMaterial {
                base_color: Color::srgb(0.4, 1.0, 0.4),
                ..default()
            })));

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
            let child = commands.spawn(PlayerCamera).id();
            commands.entity(parent).add_child(child);
            child
        };

        let mut commands = commands.entity(child);
        commands.insert_if_new(Transform::from_translation(Vec3::new(
            0.0,
            config.height - config.eye_offset,
            0.0,
        )));
    });
}
