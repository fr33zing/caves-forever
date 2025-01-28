use bevy::{math::Vec3A, picking::backend::ray::RayMap, prelude::*};
use mines::{
    tnua::consts::{PLAYER_HEIGHT, PLAYER_RADIUS},
    worldgen::terrain::Chunk,
};
use transform_gizmo_bevy::{
    Color32, EnumSet, GizmoHotkeys, GizmoMode, GizmoOptions, GizmoOrientation, GizmoTarget,
    GizmoVisuals, TransformGizmoPlugin,
};

use crate::{
    mode::ModeSpecific,
    state::{EditorState, EditorViewMode, SpawnPickerMode},
};

pub struct EditorGizmosPlugin;

#[derive(Component)]
pub struct Pickable(pub Option<EnumSet<GizmoMode>>, pub Option<GizmoOrientation>);

#[derive(Component)]
pub struct SpawnPositionIndicator;

#[derive(Component)]
pub struct ConnectionPlane;

#[derive(Component)]
pub struct ConnectionPoint;

#[derive(Component)]
pub struct ConnectedPath;

impl Plugin for EditorGizmosPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((MeshPickingPlugin, TransformGizmoPlugin));
        app.insert_resource(MeshPickingSettings {
            require_markers: true,
            ray_cast_visibility: RayCastVisibility::VisibleInView,
        });
        app.insert_resource(GizmoOptions {
            visuals: GizmoVisuals {
                x_color: Color32::from_rgb(250, 70, 70),
                y_color: Color32::from_rgb(70, 250, 70),
                z_color: Color32::from_rgb(70, 70, 250),
                inactive_alpha: 0.2,
                highlight_alpha: 0.6,
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
                pick,
                pick_spawn_position,
                draw_spawn_position,
                draw_connection_planes,
                draw_connection_points,
            ),
        );
    }
}

fn pick(
    mut commands: Commands,
    mut ray_cast: MeshRayCast,
    ray_map: Res<RayMap>,
    state: Res<EditorState>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut gizmo_options: ResMut<GizmoOptions>,
    pickables: Query<(Entity, &Pickable)>,
    gizmo_targets: Query<(Entity, &GizmoTarget)>,
) {
    if state.spawn.mode != SpawnPickerMode::Inactive {
        return;
    }
    if !mouse.just_released(MouseButton::Left) {
        return;
    }
    if gizmo_targets.iter().any(|(_, target)| target.is_focused()) {
        return;
    }

    let mut miss = true;

    for (_, ray) in ray_map.iter() {
        let settings = RayCastSettings {
            filter: &|entity| pickables.get(entity).is_ok(),
            ..default()
        };

        let Some((entity, _)) = ray_cast.cast_ray(*ray, &settings).first() else {
            continue;
        };
        let Ok((_, pickable)) = pickables.get(*entity) else {
            continue;
        };

        gizmo_targets.iter().for_each(|(entity, _)| {
            commands.entity(entity).remove::<GizmoTarget>();
        });
        commands.entity(*entity).insert(GizmoTarget::default());

        gizmo_options.gizmo_modes = pickable.0.unwrap_or_else(|| GizmoMode::all());
        gizmo_options.gizmo_orientation = pickable.1.unwrap_or_else(|| GizmoOrientation::default());

        miss = false;
        break;
    }

    if miss {
        gizmo_targets.iter().for_each(|(entity, _)| {
            commands.entity(entity).remove::<GizmoTarget>();
        });
    }
}

fn pick_spawn_position(
    mut ray_cast: MeshRayCast,
    ray_map: Res<RayMap>,
    mut state: ResMut<EditorState>,
    mouse: Res<ButtonInput<MouseButton>>,
    chunks: Query<Entity, With<Chunk>>,
) {
    if state.spawn.mode != SpawnPickerMode::Picking {
        return;
    }

    let mut spawn_pos: Option<Vec3> = None;

    for (_, ray) in ray_map.iter() {
        let settings = RayCastSettings {
            filter: &|entity| chunks.get(entity).is_ok(),
            ..default()
        };

        let Some((_, hit)) = ray_cast.cast_ray(*ray, &settings).first() else {
            continue;
        };

        spawn_pos = Some(hit.point + hit.normal * 0.1);
        break;
    }

    state.spawn.position = spawn_pos;

    if mouse.just_released(MouseButton::Left) {
        state.spawn.mode = SpawnPickerMode::Spawning;
    }
}

fn draw_spawn_position(
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
                ModeSpecific(state.mode(), Some(EditorViewMode::Preview)),
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

fn draw_connection_planes(
    mut gizmos: Gizmos,
    state: Res<EditorState>,
    planes: Query<(&Transform, Option<&GizmoTarget>), With<ConnectionPlane>>,
) {
    if state.spawn.mode == SpawnPickerMode::Playing {
        return;
    };

    planes.iter().for_each(
        |(
            Transform {
                translation,
                rotation,
                scale,
            },
            selected,
        )| {
            let color = if selected.is_some() {
                Color::srgb(0.0, 1.0, 1.0)
            } else {
                Color::srgb(1.0, 1.0, 1.0)
            };

            let isometry = Isometry3d {
                translation: Vec3A::new(translation.x, translation.y, translation.z),
                rotation: *rotation
                    * Quat::from_euler(EulerRot::XYZ, 90.0_f32.to_radians(), 0.0, 0.0),
            };
            gizmos.rect(isometry, scale.xz(), color);

            let t = Transform::from_translation(*translation).with_rotation(*rotation);
            let end = t.transform_point(Vec3::Y * 2.0);
            gizmos.arrow(*translation, end, color);
        },
    );
}

fn draw_connection_points(
    mut gizmos: Gizmos,
    state: Res<EditorState>,
    camera: Single<&Transform, With<Camera3d>>,
    points: Query<(&GlobalTransform, Option<&Pickable>), With<ConnectionPoint>>,
) {
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
