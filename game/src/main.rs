use avian3d::prelude::*;
use bevy::{ecs::world::CommandQueue, prelude::*, window::PresentMode};
use bevy_egui::EguiPlugin;
use bevy_rand::{plugin::EntropyPlugin, prelude::WyRand};
use noisy_bevy::NoisyShaderPlugin;

use lib::{
    debug_aim::DebugAimPlugin,
    materials::{CaveMaterial, LineMaterialPlugin},
    player::{PlayerPlugin, SpawnPlayerCommand},
    worldgen::{
        layout::{self, InitLayoutCommand, LayoutPlugin},
        terrain::TerrainPlugin,
    },
};

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    title: "Caves Forever".to_string(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: "../assets".to_owned(),
                ..default()
            }),
    );

    app.add_plugins((
        EguiPlugin,
        PhysicsPlugins::default(),
        LineMaterialPlugin,
        NoisyShaderPlugin,
        EntropyPlugin::<WyRand>::default(),
    ));

    app.add_plugins((
        LayoutPlugin,
        TerrainPlugin,
        MaterialPlugin::<CaveMaterial>::default(),
        PlayerPlugin,
        // debug
        DebugAimPlugin,
    ));

    app.add_systems(Startup, setup.after(layout::setup_state));

    app.run();
}

fn setup(mut commands: Commands) {
    commands.insert_resource(AmbientLight {
        color: Color::srgb(1.0, 1.0, 1.0).into(),
        brightness: 35.0,
    });

    commands.queue(InitLayoutCommand {
        after: {
            let mut queue = CommandQueue::default();
            queue.push(SpawnPlayerCommand::default());
            queue
        },
    });
}
