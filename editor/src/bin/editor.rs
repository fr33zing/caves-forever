use avian3d::prelude::*;
use bevy::{
    pbr::{wireframe::WireframePlugin, ExtendedMaterial},
    prelude::*,
    render::{
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
        view::RenderLayers,
        RenderPlugin,
    },
    window::PresentMode,
};
use bevy_egui::EguiPlugin;
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridPlugin};
use bevy_rand::{plugin::EntropyPlugin, prelude::WyRand};
use bevy_trackball::TrackballPlugin;
use noisy_bevy::NoisyShaderPlugin;

use editor_lib::{
    gizmos::EditorGizmosPlugin, mode::EditorModesPlugin, picking::PickingPlugin,
    state::EditorState, ui::EditorUiPlugin,
};
use lib::{
    materials::{CaveMaterialExtension, LineMaterialPlugin},
    player::PlayerPlugin,
    render_layer,
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
                    title: "Editor".to_string(),
                    ..default()
                }),
                ..default()
            })
            .set(AssetPlugin {
                file_path: "../assets".to_owned(),
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

    app.add_plugins((
        WireframePlugin,
        EguiPlugin,
        PhysicsPlugins::default(),
        PhysicsDebugPlugin::default(),
        LineMaterialPlugin,
        NoisyShaderPlugin,
        InfiniteGridPlugin,
        TrackballPlugin,
    ));

    app.add_plugins((
        TerrainPlugin,
        PlayerPlugin,
        MaterialPlugin::<ExtendedMaterial<StandardMaterial, CaveMaterialExtension>>::default(),
    ));

    app.init_resource::<EditorState>();
    app.add_plugins((
        EditorUiPlugin,
        EditorModesPlugin,
        EditorGizmosPlugin,
        PickingPlugin,
    ));

    // DEBUG
    app.add_plugins(LayoutPlugin);
    app.add_plugins(EntropyPlugin::<WyRand>::default());
    // DEBUG

    app.add_systems(Startup, setup);
    app.add_systems(Startup, init_layout.after(layout::setup_state)); //TEMP

    app.run();
}

fn setup(mut commands: Commands) {
    commands.spawn((
        InfiniteGridBundle { ..default() },
        RenderLayers::from_layers(&[render_layer::EDITOR, render_layer::EDITOR_PREVIEW]),
    ));

    commands.insert_resource(AmbientLight {
        color: Color::srgb(1.0, 1.0, 1.0).into(),
        brightness: 100.0,
    });
}

fn init_layout(mut commands: Commands) {
    commands.queue(InitLayoutCommand { after: default() });
}
