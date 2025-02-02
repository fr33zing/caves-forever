use bevy::{math::Vec3A, prelude::*};
use transform_gizmo_bevy::{
    Color32, GizmoHotkeys, GizmoOptions, GizmoTarget, GizmoVisuals, TransformGizmoPlugin,
};

use crate::{
    data::{RoomPartPayload, RoomPartUuid},
    mode::{EditorGizmos, ModeSpecific},
    picking::{Placing, PrimarySelection, Selectable},
    state::{EditorState, EditorViewMode, FilePayload, SpawnPickerMode},
};
use lib::{
    player::consts::{PLAYER_HEIGHT, PLAYER_RADIUS},
    worldgen::asset::PortalDirection,
};

pub struct EditorGizmosPlugin;

/// This is used for the playtest function, not real spawnpoints.
#[derive(Component)]
pub struct SpawnPositionIndicator;

#[derive(Component)]
pub struct SpawnpointGizmos;

#[derive(Component)]
pub struct PortalGizmos;

#[derive(Component)]
pub struct ConnectionPoint;

#[derive(Component)]
pub struct ConnectedPath;

impl Plugin for EditorGizmosPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TransformGizmoPlugin);
        app.insert_resource(GizmoOptions {
            visuals: GizmoVisuals {
                x_color: Color32::from_rgb(250, 70, 70),
                y_color: Color32::from_rgb(70, 250, 70),
                z_color: Color32::from_rgb(70, 70, 250),
                inactive_alpha: 0.7,
                highlight_alpha: 1.0,
                stroke_width: 3.0,
                gizmo_size: 70.0,
                ..default()
            },
            hotkeys: Some(GizmoHotkeys::default()),
            ..default()
        });

        app.add_systems(
            Update,
            (
                draw_playtest_spawn_position,
                draw_spawnpoints,
                draw_portals,
                draw_connection_points,
            ),
        );
    }
}

fn draw_playtest_spawn_position(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    state: Res<EditorState>,
    spawn_pos_indicator: Option<Single<Entity, With<SpawnPositionIndicator>>>,
) {
    if let Some(spawn_pos) = state.spawn.position {
        let mut commands = if let Some(spawn_pos_indicator) = spawn_pos_indicator {
            commands.entity(*spawn_pos_indicator)
        } else {
            commands.spawn((
                SpawnPositionIndicator,
                Mesh3d(meshes.add(Capsule3d::new(
                    PLAYER_RADIUS,
                    (PLAYER_HEIGHT - PLAYER_RADIUS * 2.0) / 2.0,
                ))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(0.0, 1.0, 0.0),
                    ..default()
                })),
            ))
        };

        let transform = Transform::from_translation(spawn_pos + (Vec3::Y * PLAYER_HEIGHT / 2.0));
        commands.insert(transform);
    } else {
        if let Some(spawn_pos_indicator) = spawn_pos_indicator {
            commands.entity(*spawn_pos_indicator).clear();
        }
    }
}

fn draw_spawnpoints(
    mut gizmos: Gizmos<EditorGizmos>,
    spawnpoints: Query<&Transform, With<SpawnpointGizmos>>,
) {
    spawnpoints.iter().for_each(|spawnpoint| {
        let color = Color::srgb(0.0, 0.75, 0.0);
        gizmos.circle(
            Isometry3d {
                translation: spawnpoint.translation.into(),
                rotation: spawnpoint.rotation
                    * Quat::from_euler(EulerRot::XYZ, 90.0_f32.to_radians(), 0.0, 0.0),
            },
            PLAYER_RADIUS,
            color,
        );

        let start = spawnpoint.translation;
        let end = start + spawnpoint.forward() * PLAYER_RADIUS * 3.0;
        gizmos.arrow(start, end, color);
    });
}

fn draw_portals(
    mut gizmos: Gizmos,
    state: Res<EditorState>,
    planes: Query<
        (
            Entity,
            &Transform,
            Option<&PrimarySelection>,
            Option<&GizmoTarget>,
            Option<&RoomPartUuid>,
        ),
        With<PortalGizmos>,
    >,
    placing: Query<Entity, With<Placing>>,
) {
    if state.spawn.mode == SpawnPickerMode::Playing {
        return;
    };

    planes.iter().for_each(
        |(
            entity,
            Transform {
                translation,
                rotation,
                scale,
            },
            primary,
            selected,
            uuid,
        )| {
            // TODO add something like GizmoColorIndicatesSelection
            let color = if selected.is_some() {
                if primary.is_some() {
                    Color::srgb(0.0, 1.0, 1.0)
                } else {
                    Color::srgb(0.0, 0.4, 1.0)
                }
            } else {
                Color::srgb(1.0, 1.0, 1.0)
            };

            let isometry = Isometry3d {
                translation: Vec3A::new(translation.x, translation.y, translation.z),
                rotation: *rotation
                    * Quat::from_euler(EulerRot::XYZ, 90.0_f32.to_radians(), 0.0, 0.0),
            };
            gizmos.rect(isometry, scale.xz(), color);

            if placing.get(entity).is_ok() {
                return;
            }

            let bidirectional = 'bd: {
                let Some(uuid) = uuid else {
                    break 'bd false;
                };
                let Some(data) = state.files.current_data() else {
                    break 'bd false;
                };
                let FilePayload::Room(data) = data else {
                    break 'bd false;
                };
                let Some(part) = data.parts.get(&uuid.0) else {
                    break 'bd false;
                };
                let RoomPartPayload::Portal { direction } = part.data else {
                    break 'bd false;
                };

                direction == PortalDirection::Bidirectional
            };

            // Upward arrow
            let t = Transform::from_translation(*translation).with_rotation(*rotation);
            let arrow_len: f32 = 2.0;
            let end = t.transform_point(scale.z / 2.0 * Vec3::Z);
            let start = t.transform_point((scale.z / 2.0 - arrow_len) * Vec3::Z);
            gizmos.arrow(start, end, color);

            // Inward/outward arrow
            let arrow_len = 6.0;
            let start = t.transform_point(arrow_len / 2.0 * Vec3::NEG_Y);
            let end = t.transform_point(arrow_len / 2.0 * Vec3::Y);
            gizmos.arrow(start, end, color);

            if bidirectional {
                gizmos.arrow(end, start, color);
            }
        },
    );
}

fn draw_connection_points(
    mut gizmos: Gizmos,
    state: Res<EditorState>,
    camera: Query<&Transform, With<Camera3d>>,
    points: Query<(&GlobalTransform, Option<&Selectable>), With<ConnectionPoint>>,
) {
    let Ok(camera) = camera.get_single() else {
        return;
    };
    if state.spawn.mode == SpawnPickerMode::Playing {
        return;
    };

    points.iter().for_each(|(transform, pickable)| {
        if pickable.is_some() {
            return;
        }

        let color = Color::srgb(0.7, 0.7, 0.7);
        let translation = transform.translation();
        let isometry = Isometry3d {
            translation: translation.into(),
            rotation: Transform::from_translation(translation)
                .looking_at(camera.translation, Vec3::Y)
                .rotation,
        };

        gizmos.circle(isometry, 0.5, color);
    });
}
