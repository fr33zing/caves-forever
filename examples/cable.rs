use avian3d::{math::Scalar, prelude::*};
use bevy::{
    pbr::wireframe::{Wireframe, WireframeConfig, WireframePlugin},
    prelude::*,
    render::{
        mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes},
        settings::{RenderCreation, WgpuFeatures, WgpuSettings},
        view::NoFrustumCulling,
        RenderPlugin,
    },
    window::PresentMode,
};

use mines::{
    cable::{
        generate_colliders, generate_mesh as generate_cable_mesh, CableEnd, CablePlugin,
        CableSegment, CableSegments, CableSkinnedMeshJoint, CableStart,
    },
    physics::GameLayer,
    player::{
        camera::PlayerCameraPlugin,
        motion::{CharacterController, CharacterControllerBundle, CharacterControllerPlugin},
    },
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
            }),
        WireframePlugin,
    ))
    .insert_resource(WireframeConfig {
        global: false,
        default_color: bevy::color::palettes::css::WHITE.into(),
    });

    app.add_plugins((PhysicsPlugins::default(), PhysicsDebugPlugin::default()));

    app.add_plugins((CablePlugin, PlayerCameraPlugin, CharacterControllerPlugin));

    app.add_systems(Startup, (setup_world, setup_player, setup_cable).chain());

    app.run();
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
        Collider::cylinder(radius, height),
        CollisionLayers::new(GameLayer::World, [GameLayer::all_bits()]),
        Mesh3d(meshes.add(Cylinder::new(radius, height).mesh().resolution(32))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.4, 0.4),
            ..default()
        })),
    ));

    // Test cube
    commands.spawn((
        Transform::from_xyz(0.0, 1.0, 8.0),
        RigidBody::Static,
        Collider::cuboid(1.0, 1.0, 1.0),
        CollisionLayers::new(GameLayer::World, [GameLayer::all_bits()]),
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0).mesh())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.4, 0.4, 1.0),
            ..default()
        })),
    ));
}

fn setup_player(mut commands: Commands) {
    commands.spawn((
        Transform::from_xyz(0.0, 4.0, 0.0),
        CharacterControllerBundle::new(
            Collider::capsule(0.4, 1.0),
            Vec3::new(0.0, -9.81 * 2.0, 0.0),
        )
        .with_movement(1000.0, 0.90, 6.0, (50.0 as Scalar).to_radians()),
        Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
        Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
        GravityScale(2.0),
    ));
}

fn setup_cable(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut skinned_mesh_inverse_bindposes: ResMut<Assets<SkinnedMeshInverseBindposes>>,
    player: Query<Entity, With<CharacterController>>,
) {
    let max_length = 16.0;
    let segment_config = CableSegments {
        length: 0.2,
        radius: 0.02,
        faces: 3,
    };
    let (mesh, inverse_bindposes) = generate_cable_mesh(max_length, &segment_config);
    let colliders = generate_colliders(max_length, &segment_config);
    let mut physics_joints = Vec::<SphericalJoint>::new();
    let mut prev_segment: Option<Entity> = None;
    let mut segments = Vec::<Entity>::new();
    let mut skinned_mesh_joints = Vec::<Entity>::new();

    commands
        .spawn((
            CableStart,
            CableSegment,
            Transform::from_xyz(0.0, 0.0, -8.0),
        ))
        .with_children(|parent| {
            let len = colliders.len();
            for (i, (collider, y)) in colliders.into_iter().enumerate() {
                let entity = parent
                    .spawn((
                        CableSegment,
                        collider,
                        RigidBody::Dynamic,
                        AngularDamping(8.0),
                        LinearDamping(2.0),
                        CollisionLayers::new(
                            GameLayer::Cable,
                            [GameLayer::Cable, GameLayer::World],
                        ),
                        Transform::from_xyz(0.0, y, 0.0),
                        DebugRender::default().without_axes(),
                    ))
                    .with_children(|parent| {
                        if i == len - 1 {
                            let end = parent
                                .spawn((CableEnd, CableSegment, Transform::IDENTITY))
                                .id();

                            segments.push(end);
                        }
                    })
                    .id();

                segments.push(entity);

                // Create physics joints to connect previous segment
                if let Some(prev) = prev_segment {
                    let joint = SphericalJoint::new(prev, entity)
                        .with_compliance(0.4)
                        .with_angular_velocity_damping(16.0)
                        .with_linear_velocity_damping(16.0)
                        .with_local_anchor_1(Vec3::Y * (segment_config.length / 2.0))
                        .with_local_anchor_2(Vec3::NEG_Y * (segment_config.length / 2.0));
                    physics_joints.push(joint);
                }
                prev_segment = Some(entity);
            }
        });

    // Spawn physics joints
    for joint in physics_joints {
        commands.spawn(joint);
    }

    // Spawn skinned mesh joints
    for segment in segments {
        let joint = commands
            .spawn((
                CableSkinnedMeshJoint(segment),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ))
            .id();

        skinned_mesh_joints.push(joint);
    }

    // Spawn cable mesh
    commands.spawn((
        //Wireframe,
        NoFrustumCulling, // TODO figure out why this is necessary
        Transform::from_xyz(0.0, 0.0, -8.0),
        Mesh3d(meshes.add(mesh)),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.4, 0.4),
            ..default()
        })),
        SkinnedMesh {
            inverse_bindposes: skinned_mesh_inverse_bindposes.add(inverse_bindposes),
            joints: skinned_mesh_joints,
        },
    ));

    // Attach to player
    if let Ok(player) = player.get_single() {
        if let Some(last_segment) = prev_segment {
            commands.spawn(
                DistanceJoint::new(last_segment, player)
                    .with_compliance(0.9)
                    .with_local_anchor_1(Vec3::Y * (segment_config.length / 2.0))
                    .with_local_anchor_2(Vec3::new(0.0, 1.0, -4.0)),
            );
        }
    }
}
