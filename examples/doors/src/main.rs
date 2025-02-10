use avian3d::prelude::*;
use bevy::{
    asset::{processor::LoadTransformAndSave, transformer::IdentityAssetTransformer},
    audio::{AudioPlugin, SpatialScale},
    image::{CompressedImageSaver, ImageAddressMode, ImageFilterMode, ImageLoader},
    pbr::wireframe::{WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
        RenderPlugin,
    },
    window::PresentMode,
};
use bevy_egui::EguiPlugin;
use bevy_rand::{plugin::EntropyPlugin, prelude::WyRand};
use lib::{
    meshgen::{AddDoorwayToEntity, DoorwaySpec, MeshGenerationPlugin},
    physics::GameLayer,
    player::{PlayerPlugin, SpawnPlayerCommand},
};

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
        EntropyPlugin::<WyRand>::default(),
    ));

    app.add_plugins((MeshGenerationPlugin, PlayerPlugin));

    app.add_systems(Startup, (setup_world, setup_player).chain());
    app.add_systems(Update, fixup_images);

    app.run();
}

fn fixup_images(mut ev_asset: EventReader<AssetEvent<Image>>, mut assets: ResMut<Assets<Image>>) {
    for ev in ev_asset.read() {
        match ev {
            AssetEvent::LoadedWithDependencies { id } => {
                let texture = assets.get_mut(*id).unwrap();
                let descriptor = texture.sampler.get_or_init_descriptor();
                descriptor.address_mode_u = ImageAddressMode::Repeat;
                descriptor.address_mode_v = ImageAddressMode::Repeat;
                descriptor.mipmap_filter = ImageFilterMode::Linear;
                descriptor.min_filter = ImageFilterMode::Linear;
            }
            _ => {}
        }
    }
}

fn setup_world(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    commands.insert_resource(AmbientLight {
        color: Color::srgb(1.0, 1.0, 1.0).into(),
        brightness: 800.0,
    });

    // Ground
    let (radius, height) = (64.0, 1.0);
    commands.spawn((
        RigidBody::Static,
        Transform::from_translation(Vec3::NEG_Y * (height / 2.0)),
        Collider::cylinder(radius, height),
        CollisionLayers::new(GameLayer::World, [GameLayer::all_bits()]),
        Mesh3d(meshes.add(Cylinder::new(radius, height).mesh().resolution(32))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.4, 0.4),
            ..default()
        })),
    ));

    let frame_width = 6.0;
    let frame_height = 4.0;
    let door_width = 2.75;
    let door_height = 2.25;
    let door_offset = (0.6, 0.15);
    let doorway = commands.spawn(Transform::default()).id();
    commands.queue(AddDoorwayToEntity {
        spec: DoorwaySpec {
            frame: Rect {
                min: Vec2::new(-frame_width / 2.0, 0.0),
                max: Vec2::new(frame_width / 2.0, frame_height),
            },
            door: Rect {
                min: Vec2::new(-door_width / 2.0 + door_offset.0, door_offset.1),
                max: Vec2::new(
                    door_width / 2.0 + door_offset.0,
                    door_offset.1 + door_height,
                ),
            },
            frame_depth: 0.4,
            door_depth: 0.075,
            frame_uv_scale: 4.0,
            door_uv_scale: 4.0,
        },
        entity: doorway,
    });
}

fn setup_player(mut commands: Commands) {
    commands.queue(SpawnPlayerCommand {
        position: Some(Vec3::new(0.0, 2.0, 8.0)),
    });
}
