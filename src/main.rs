use avian3d::prelude::*;
use bevy::pbr::wireframe::{WireframeConfig, WireframePlugin};
use bevy::render::settings::{RenderCreation, WgpuFeatures, WgpuSettings};
use bevy::render::RenderPlugin;
use bevy::window::PresentMode;
use bevy::{pbr::ExtendedMaterial, prelude::*};
use mines::debug_aim::DebugAimPlugin;
use noisy_bevy::NoisyShaderPlugin;

use mines::{
    materials::{CaveMaterialExtension, LineMaterialPlugin},
    tnua::PlayerPlugin,
    worldgen::terrain::TerrainPlugin,
};

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            })
            .set(RenderPlugin {
                render_creation: RenderCreation::Automatic(WgpuSettings {
                    features: WgpuFeatures::POLYGON_MODE_LINE,
                    ..default()
                }),
                ..default()
            }),
    );

    app.insert_resource(WireframeConfig {
        global: false,
        default_color: bevy::color::palettes::css::WHITE.into(),
    });

    app.add_plugins((
        WireframePlugin,
        PhysicsPlugins::default(),
        LineMaterialPlugin,
        NoisyShaderPlugin,
    ));

    app.add_plugins((
        TerrainPlugin,
        MaterialPlugin::<ExtendedMaterial<StandardMaterial, CaveMaterialExtension>>::default(),
        PlayerPlugin,
        // debug
        DebugAimPlugin,
    ));

    app.add_systems(Startup, setup);

    app.run();
}

fn setup(mut commands: Commands) {
    commands.insert_resource(AmbientLight {
        color: Color::srgb(1.0, 1.0, 1.0).into(),
        brightness: 25.0,
    });
}
