pub mod player;

use avian3d::prelude::*;
use bevy::{
    asset::{processor::LoadTransformAndSave, transformer::IdentityAssetTransformer},
    audio::{AudioPlugin, SpatialScale},
    image::{CompressedImageSaver, ImageLoader},
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
        view::RenderLayers,
        RenderPlugin,
    },
    window::PresentMode,
};
use bevy_egui::EguiPlugin;
use lib::{
    render_layer,
    weapon::{weapons, PlayerWeapons, ViewModelCamera, WeaponPickup, WeaponPlugin, WeaponSlots},
};
use player::{Player, PlayerCamera, PlayerPlugin};

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
            })
            .set(AssetPlugin {
                file_path: "../../assets".to_owned(),
                processed_file_path: "../../imported_assets".to_owned(),
                mode: AssetMode::Processed,
                ..default()
            })
            .set(AudioPlugin {
                default_spatial_scale: SpatialScale::new(1.0 / 16.0),
                ..default()
            }),
        WireframePlugin,
    ))
    .insert_resource(WireframeConfig {
        global: false,
        default_color: bevy::color::palettes::css::WHITE.into(),
    });
    app.set_default_asset_processor::<LoadTransformAndSave<ImageLoader, IdentityAssetTransformer<_>, CompressedImageSaver>>("tga");

    app.add_plugins((
        EguiPlugin,
        PhysicsPlugins::default(),
        //PhysicsDebugPlugin::default(),
    ));

    app.add_plugins((PlayerPlugin, WeaponPlugin));

    app.add_systems(Startup, (setup_world, setup_player).chain());
    app.add_systems(Update, setup_collider);

    app.run();
}

fn setup_world(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(AmbientLight {
        color: Color::srgb(1.0, 1.0, 1.0).into(),
        brightness: 600.0,
    });

    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/kcc.glb")),
    ));

    commands.spawn((
        RenderLayers::from_layers(&[render_layer::WORLD, render_layer::VIEW_MODEL]),
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 5000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_translation(Vec3::ONE * 512.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Transform::from_translation(Vec3::Z * -4.0),
        WeaponPickup::new(&weapons::SHOTGUN),
    ));
}

fn setup_collider(
    mut commands: Commands,
    mut ev_asset: EventReader<AssetEvent<Mesh>>,
    meshes: Res<Assets<Mesh>>,
) {
    for ev in ev_asset.read() {
        let AssetEvent::LoadedWithDependencies { id } = ev else {
            continue;
        };
        commands.spawn((
            Transform::default(),
            RigidBody::Static,
            Collider::trimesh_from_mesh(meshes.get(*id).unwrap()).unwrap(),
        ));
    }
}

fn setup_player(mut commands: Commands) {
    let mut viewmodel_camera = Entity::PLACEHOLDER;
    commands.spawn(PlayerCamera).with_children(|parent| {
        viewmodel_camera = parent.spawn(ViewModelCamera::default()).id();
    });

    commands.spawn((
        Player,
        WeaponSlots::new(1),
        PlayerWeapons { viewmodel_camera },
        Transform::from_translation(Vec3::Y * 1.0),
    ));
}
