use avian3d::prelude::*;
use bevy::{
    pbr::{
        wireframe::{WireframeConfig, WireframePlugin},
        ExtendedMaterial,
    },
    prelude::*,
    render::{
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
    window::PresentMode,
};
use mines::{
    debug_camera::DebugCameraPlugin,
    materials::{CaveMaterialExtension, LineMaterialPlugin},
    worldgen::terrain::TerrainPlugin,
};
use noisy_bevy::NoisyShaderPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins((
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
        WireframePlugin,
    ))
    .insert_resource(WireframeConfig {
        global: false,
        default_color: bevy::color::palettes::css::WHITE.into(),
    });

    app.add_plugins((
        PhysicsPlugins::default(),
        PhysicsDebugPlugin::default(),
        LineMaterialPlugin,
        NoisyShaderPlugin,
    ));

    app.add_plugins((
        MaterialPlugin::<ExtendedMaterial<StandardMaterial, CaveMaterialExtension>>::default(),
        TerrainPlugin,
        DebugCameraPlugin,
    ));

    app.add_systems(Startup, setup);
    app.run();
}

fn setup(mut commands: Commands) {
    commands.insert_resource(AmbientLight {
        color: Color::srgb(1.0, 1.0, 1.0).into(),
        brightness: 50.0,
    });
}
