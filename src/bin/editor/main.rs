use avian3d::prelude::*;
use bevy::{asset::AssetMetaCheck, pbr::ExtendedMaterial, prelude::*, window::PresentMode};
use bevy_egui::EguiPlugin;
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridPlugin};
use bevy_trackball::TrackballPlugin;
use gizmos::EditorGizmosPlugin;
use mode::EditorModesPlugin;
use noisy_bevy::NoisyShaderPlugin;

use mines::{
    materials::{CaveMaterialExtension, LineMaterialPlugin},
    worldgen::terrain::TerrainPlugin,
};

mod camera;
mod gizmos;
mod mode;
mod state;
mod ui;
mod util;

use state::EditorState;
use ui::EditorUiPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    title: "Editor".to_string(),
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
        InfiniteGridPlugin,
        TrackballPlugin,
    ));

    app.add_plugins((
        TerrainPlugin,
        MaterialPlugin::<ExtendedMaterial<StandardMaterial, CaveMaterialExtension>>::default(),
    ));

    app.init_resource::<EditorState>();
    app.add_plugins((EditorUiPlugin, EditorModesPlugin, EditorGizmosPlugin));

    app.add_systems(Startup, setup);

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn(InfiniteGridBundle { ..default() });

    commands.insert_resource(AmbientLight {
        color: Color::srgb(1.0, 1.0, 1.0).into(),
        brightness: 35.0,
    });
}
