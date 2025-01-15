use avian3d::prelude::*;
use bevy::{asset::AssetMetaCheck, pbr::ExtendedMaterial, prelude::*, window::PresentMode};
use bevy_egui::EguiPlugin;
use noisy_bevy::NoisyShaderPlugin;

use mines::{
    debug_aim::DebugAimPlugin,
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
                    title: "Caves Forever".to_string(),
                    canvas: Some("#bevy".to_owned()),
                    fit_canvas_to_parent: true,
                    prevent_default_event_handling: false,
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                meta_check: AssetMetaCheck::Never,
                ..default()
            }),
    );

    app.add_plugins((
        EguiPlugin,
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
