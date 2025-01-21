use bevy::prelude::*;

use mines::{
    tnua::consts::{PLAYER_HEIGHT, PLAYER_RADIUS},
    worldgen::consts::CHUNK_SIZE_F,
};

use crate::{state::EditorMode, util::mesh_text};

use super::ModeSpecific;

pub fn enter(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // "Player"
    commands.spawn((
        ModeSpecific(EditorMode::Tunnels, None),
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -90.0_f32.to_radians(),
            0.0,
            0.0,
        ))
        .with_translation(Vec3::new(
            -PLAYER_RADIUS + 0.017,
            0.0,
            -PLAYER_HEIGHT / 2.0 + 0.14,
        ))
        .with_scale(Vec3::splat(0.2)),
        Mesh3d(meshes.add(mesh_text("Player", true))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 1.0, 0.0),
            unlit: true,
            ..default()
        })),
    ));

    // "Chunk"
    commands.spawn((
        ModeSpecific(EditorMode::Tunnels, None),
        Transform::from_rotation(Quat::from_euler(
            EulerRot::XYZ,
            -90.0_f32.to_radians(),
            0.0,
            0.0,
        ))
        .with_translation(Vec3::new(
            -CHUNK_SIZE_F / 2.0 + 0.2,
            0.0,
            -CHUNK_SIZE_F / 2.0 + 1.6,
        ))
        .with_scale(Vec3::splat(2.25)),
        Mesh3d(meshes.add(mesh_text("Chunk", true))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 1.0),
            unlit: true,
            ..default()
        })),
    ));
}

pub fn update(mut gizmos: Gizmos) {
    // Player
    gizmos.rounded_cuboid(
        Vec3::ZERO,
        Vec3::new(PLAYER_RADIUS * 2.0, 0.0, PLAYER_HEIGHT),
        Color::srgb(0.243, 0.757, 0.176),
    );

    // Chunk
    gizmos.rounded_cuboid(
        Vec3::ZERO,
        Vec3::new(CHUNK_SIZE_F, 0.0, CHUNK_SIZE_F),
        Color::srgb(0.776, 0.294, 0.769),
    );
}
