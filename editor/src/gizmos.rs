use bevy::{
    math::Vec3A, pbr::wireframe::WireframeColor, picking::backend::ray::RayMap, prelude::*,
};
use transform_gizmo_bevy::{
    Color32, GizmoHotkeys, GizmoOptions, GizmoTarget, GizmoVisuals, TransformGizmoPlugin,
};

use crate::{
    data::{RoomPart, RoomPartPayload, RoomPartUuid},
    mode::ModeSpecific,
    state::{EditorState, EditorViewMode, FilePayload, SpawnPickerMode},
    ui::CursorOverEguiPanel,
};
use lib::{
    player::consts::{PLAYER_HEIGHT, PLAYER_RADIUS},
    worldgen::{asset::PortalDirection, terrain::Chunk},
};

pub struct EditorGizmosPlugin;

#[derive(Component)]
pub struct SpawnPositionIndicator;

#[derive(Component)]
pub struct PortalGizmos;

#[derive(Component)]
pub struct ConnectionPoint;

#[derive(Component)]
pub struct ConnectedPath;

#[derive(Component)]
pub struct MaterialIndicatesSelection;

#[derive(Component)]
pub struct WireframeIndicatesSelection;

#[derive(Component)]
pub struct Selectable;

#[derive(Component)]
pub struct PrimarySelection;

#[derive(Resource)]
pub struct SelectionMaterials {
    unselected: Handle<StandardMaterial>,
    selected: Handle<StandardMaterial>,
    multiselected: Handle<StandardMaterial>,
}

#[allow(unused)]
impl SelectionMaterials {
    pub fn unselected(&self) -> MeshMaterial3d<StandardMaterial> {
        MeshMaterial3d(self.unselected.clone())
    }
    pub fn selected(&self) -> MeshMaterial3d<StandardMaterial> {
        MeshMaterial3d(self.selected.clone())
    }
    pub fn multiselected(&self) -> MeshMaterial3d<StandardMaterial> {
        MeshMaterial3d(self.multiselected.clone())
    }
}

#[derive(Resource)]
pub struct SelectionWireframeColors {
    unselected: WireframeColor,
    selected: WireframeColor,
    multiselected: WireframeColor,
}
#[allow(unused)]
impl SelectionWireframeColors {
    pub fn unselected(&self) -> WireframeColor {
        self.unselected.clone()
    }
    pub fn selected(&self) -> WireframeColor {
        self.selected.clone()
    }
    pub fn multiselected(&self) -> WireframeColor {
        self.multiselected.clone()
    }
}

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
                inactive_alpha: 0.7,
                highlight_alpha: 1.0,
                stroke_width: 3.0,
                gizmo_size: 70.0,
                ..default()
            },
            hotkeys: Some(GizmoHotkeys::default()),
            ..default()
        });

        app.add_systems(Startup, setup_selection_indications);
        app.add_systems(
            Update,
            (
                pick,
                pick_spawn_position,
                update_selection_indications
                    .after(pick)
                    .after(pick_spawn_position),
                draw_spawn_position,
                draw_portals,
                draw_connection_points,
            ),
        );
    }
}

fn setup_selection_indications(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let unselected = Color::srgba(1.0, 1.0, 1.0, 0.6);
    let selected = Color::srgba(0.0, 1.0, 1.0, 0.6);
    let multiselected = Color::srgba(0.0, 0.4, 1.0, 0.6);

    commands.insert_resource(SelectionMaterials {
        unselected: materials.add(StandardMaterial {
            base_color: unselected,
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        }),
        selected: materials.add(StandardMaterial {
            base_color: selected,
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        }),
        multiselected: materials.add(StandardMaterial {
            base_color: multiselected,
            alpha_mode: AlphaMode::Add,
            unlit: true,
            ..default()
        }),
    });
    commands.insert_resource(SelectionWireframeColors {
        unselected: WireframeColor { color: unselected },
        selected: WireframeColor { color: selected },
        multiselected: WireframeColor {
            color: multiselected,
        },
    });
}

fn pick(
    mut commands: Commands,
    mut ray_cast: MeshRayCast,
    ray_map: Res<RayMap>,
    state: Res<EditorState>,
    mouse: Res<ButtonInput<MouseButton>>,
    cursor_over_egui_panel: Res<CursorOverEguiPanel>,
    keyboard: Res<ButtonInput<KeyCode>>,
    selectable: Query<Entity, With<Selectable>>,
    gizmo_targets: Query<(Entity, &GizmoTarget)>,
    primary_selection: Query<Entity, With<PrimarySelection>>,
) {
    if cursor_over_egui_panel.0 {
        return;
    }
    if state.spawn.mode != SpawnPickerMode::Inactive {
        return;
    }
    if !mouse.just_released(MouseButton::Left) {
        return;
    }
    if gizmo_targets.iter().any(|(_, target)| target.is_focused()) {
        return;
    }

    let multiselect = keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight);
    let mut miss = true;

    let deselect_all = |commands: &mut Commands| {
        if multiselect {
            return;
        };
        gizmo_targets.iter().for_each(|(entity, _)| {
            commands.entity(entity).remove::<GizmoTarget>();
        });
    };

    for (_, ray) in ray_map.iter() {
        let settings = RayCastSettings {
            filter: &|entity| selectable.get(entity).is_ok(),
            ..default()
        };

        let Some((entity, _)) = ray_cast.cast_ray(*ray, &settings).first() else {
            continue;
        };
        if selectable.get(*entity).is_err() {
            continue;
        };

        deselect_all(&mut commands);

        primary_selection.iter().for_each(|not_primary| {
            commands.entity(not_primary).remove::<PrimarySelection>();
        });

        let mut commands = commands.entity(*entity);
        commands.insert(GizmoTarget::default());
        commands.insert(PrimarySelection);

        miss = false;
        break;
    }

    if miss {
        deselect_all(&mut commands);
    }
}

fn update_selection_indications(
    mut commands: Commands,
    materials: Res<SelectionMaterials>,
    wireframes: Res<SelectionWireframeColors>,
    material_indicators: Query<Entity, With<MaterialIndicatesSelection>>,
    wireframe_indicators: Query<Entity, With<WireframeIndicatesSelection>>,
    selected: Query<Entity, With<GizmoTarget>>,
    primary_selection: Option<Single<Entity, With<PrimarySelection>>>,
) {
    fn is_primary_selection(
        entity: &Entity,
        primary_selection: &Option<Single<Entity, With<PrimarySelection>>>,
    ) -> bool {
        if let Some(primary_selection) = primary_selection {
            return *entity == **primary_selection;
        }
        false
    }

    wireframe_indicators.iter().for_each(|entity| {
        if selected.get(entity).is_ok() {
            if is_primary_selection(&entity, &primary_selection) {
                commands.entity(entity).insert(wireframes.selected.clone());
            } else {
                commands
                    .entity(entity)
                    .insert(wireframes.multiselected.clone());
            }
        } else {
            commands
                .entity(entity)
                .insert(wireframes.unselected.clone());
        }
    });

    material_indicators.iter().for_each(|entity| {
        let mut commands = commands.entity(entity);

        if selected.get(entity).is_ok() {
            if is_primary_selection(&entity, &primary_selection) {
                commands.insert(MeshMaterial3d(materials.selected.clone()));
            } else {
                commands.insert(MeshMaterial3d(materials.multiselected.clone()));
            }
        } else {
            commands.insert(MeshMaterial3d(materials.unselected.clone()));
        }
    });
}

fn pick_spawn_position(
    mut ray_cast: MeshRayCast,
    ray_map: Res<RayMap>,
    mut state: ResMut<EditorState>,
    mouse: Res<ButtonInput<MouseButton>>,
    chunks: Query<Entity, With<Chunk>>,
    cursor_over_egui_panel: Res<CursorOverEguiPanel>,
) {
    if cursor_over_egui_panel.0 {
        return;
    }
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

fn draw_portals(
    mut gizmos: Gizmos,
    state: Res<EditorState>,
    planes: Query<(&Transform, Option<&GizmoTarget>, Option<&RoomPartUuid>), With<PortalGizmos>>,
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
            uuid,
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
