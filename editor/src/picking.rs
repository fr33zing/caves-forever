use std::collections::HashMap;

use bevy::{
    ecs::system::SystemState, pbr::wireframe::WireframeColor, picking::backend::ray::RayMap,
    prelude::*, window::PrimaryWindow,
};
use bevy_trackball::TrackballCamera;
use strum::{EnumIter, IntoEnumIterator};
use transform_gizmo_bevy::GizmoTarget;

use crate::{
    data::RoomPartUuid,
    state::{EditorState, FilePayload, SpawnPickerMode},
    ui::EguiHasPointer,
};
use lib::worldgen::terrain::Chunk;

#[derive(Resource)]
pub struct SelectionMaterials {
    unselected: Handle<StandardMaterial>,
    selected: Handle<StandardMaterial>,
    multiselected: Handle<StandardMaterial>,
}
impl SelectionMaterials {
    pub fn unselected(&self) -> MeshMaterial3d<StandardMaterial> {
        MeshMaterial3d(self.unselected.clone())
    }
}

#[derive(Resource)]
pub struct SelectionWireframeColors {
    unselected: WireframeColor,
    selected: WireframeColor,
    multiselected: WireframeColor,
}
impl SelectionWireframeColors {
    pub fn unselected(&self) -> WireframeColor {
        self.unselected.clone()
    }
}

#[derive(Component)]
pub struct MaterialIndicatesSelection;

#[derive(Component)]
pub struct WireframeIndicatesSelection;

#[derive(Component)]
pub struct Selectable {
    pub order: u8,
}

#[derive(Component)]
pub struct PrimarySelection;

#[repr(u8)]
#[derive(Debug, EnumIter, PartialEq, Eq, Hash, Clone)]
pub enum PickingMode {
    Selectable,
    Terrain,
    GroundPlane,
}

#[derive(Component)]
pub struct Placing {
    pub modes: Vec<PickingMode>,
    pub align_to_hit_normal: bool,
    pub offset: Vec3,
    pub spawned_time: f64,
}

#[derive(Debug)]
pub struct PickingTarget {
    pub point: Vec3,
    pub normal: Vec3,
    pub entity: Option<Entity>,
}

#[derive(Resource, Debug)]
pub struct PickingTargets(pub HashMap<PickingMode, Option<PickingTarget>>);

impl PickingTargets {
    fn target(&self, mode: &PickingMode) -> &Option<PickingTarget> {
        self.0.get(mode).unwrap()
    }

    fn targets(&self, modes: &[PickingMode]) -> Option<&PickingTarget> {
        for mode in modes {
            if let Some(target) = self.target(mode) {
                return Some(target);
            }
        }
        None
    }
}

pub struct SpawnAndPlaceCommand<T>
where
    T: Bundle,
{
    pub modes: Vec<PickingMode>,
    pub align_to_hit_normal: bool,
    pub offset: Vec3,
    pub bundle: T,
}

impl<T> Command for SpawnAndPlaceCommand<T>
where
    T: Bundle,
{
    fn apply(self, world: &mut World) {
        let mut system_state: SystemState<(
            Res<Time>,
            Commands,
            Query<Entity, With<GizmoTarget>>,
            Option<Single<Entity, With<PrimarySelection>>>,
        )> = SystemState::new(world);
        let (time, mut commands, selected, primary) = system_state.get_mut(world);

        selected.iter().for_each(|selected| {
            let mut commands = commands.entity(selected);
            commands.remove::<GizmoTarget>();
            commands.remove::<Placing>();
        });

        if let Some(primary) = primary {
            commands.entity(*primary).remove::<PrimarySelection>();
        }

        let mut commands = commands.spawn(self.bundle);
        commands.insert(Placing {
            modes: self.modes,
            align_to_hit_normal: self.align_to_hit_normal,
            offset: self.offset,
            spawned_time: time.elapsed_secs_f64(),
        });
        commands.insert(PrimarySelection);

        system_state.apply(world);
    }
}

pub struct CancelEntityPlacement;

impl Command for CancelEntityPlacement {
    fn apply(self, world: &mut World) {
        let mut system_state: SystemState<(
            Commands,
            ResMut<EditorState>,
            Query<(Entity, &RoomPartUuid), With<Placing>>,
        )> = SystemState::new(world);
        let (mut commands, mut state, placing) = system_state.get_mut(world);

        placing.iter().for_each(|(entity, uuid)| {
            commands.entity(entity).despawn();

            let Some(data) = state.files.current_data_mut() else {
                return;
            };
            let FilePayload::Room(room) = data else {
                return;
            };
            room.parts.remove(&uuid.0);
        });

        system_state.apply(world);
    }
}

pub struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MeshPickingPlugin);
        app.insert_resource(MeshPickingSettings {
            require_markers: true,
            ray_cast_visibility: RayCastVisibility::VisibleInView,
        });

        app.insert_resource(PickingTargets(
            PickingMode::iter().map(|mode| (mode, None)).collect(),
        ));

        app.add_systems(Startup, setup_selection_indications);
        app.add_systems(
            Update,
            (
                update_picking_targets,
                ((pick, pick_spawn_position).chain(), place_new_entity),
                update_selection_indications,
            )
                .chain(),
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

fn update_picking_targets(
    mut targets: ResMut<PickingTargets>,
    mut ray_cast: MeshRayCast,
    ray_map: Res<RayMap>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<TrackballCamera>>,
    selectable: Query<(Entity, &Selectable)>,
    chunks: Query<Entity, With<Chunk>>,
    placing: Option<Single<Entity, With<Placing>>>,
    egui_has_pointer: Res<EguiHasPointer>,
) {
    if egui_has_pointer.0 {
        targets.0.iter_mut().for_each(|(_, target)| {
            *target = None;
        });

        return;
    }

    PickingMode::iter().for_each(|mode| {
        let target = match mode {
            PickingMode::Selectable => ray_map.iter().find_map(|(_, ray)| {
                let settings = RayCastSettings {
                    filter: &|entity| -> bool {
                        let not_placing = if let Some(ref placing) = placing {
                            entity != **placing
                        } else {
                            true
                        };
                        selectable.get(entity).is_ok() && not_placing
                    },
                    ..default()
                };
                let settings = settings.never_early_exit();

                let mut hits = ray_cast.cast_ray(*ray, &settings).to_vec();
                hits.sort_by_key(|hit| selectable.get(hit.0).unwrap().1.order);
                hits.first().map(|(entity, hit)| PickingTarget {
                    point: hit.point,
                    normal: hit.normal,
                    entity: Some(*entity),
                })
            }),
            PickingMode::Terrain => ray_map.iter().find_map(|(_, ray)| {
                let settings = RayCastSettings {
                    filter: &|entity| chunks.get(entity).is_ok(),
                    ..default()
                };
                ray_cast
                    .cast_ray(*ray, &settings)
                    .first()
                    .map(|(entity, hit)| PickingTarget {
                        point: hit.point,
                        normal: hit.normal,
                        entity: Some(*entity),
                    })
            }),
            PickingMode::GroundPlane => {
                cursor_to_ground_plane(&window, *camera).map(|hit| PickingTarget {
                    point: Vec3::new(hit.x, 0.0, hit.y),
                    normal: Vec3::Y,
                    entity: None,
                })
            }
        };
        targets.0.insert(mode, target);
    });
}

fn pick(
    mut commands: Commands,
    state: Res<EditorState>,
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    gizmo_targets: Query<(Entity, &GizmoTarget)>,
    primary_selection: Query<Entity, With<PrimarySelection>>,
    placing: Query<&Placing>,
    picking_targets: Res<PickingTargets>,
) {
    if !placing.is_empty() {
        return;
    };
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

    if !multiselect {
        gizmo_targets.iter().for_each(|(entity, _)| {
            commands.entity(entity).remove::<GizmoTarget>();
        });
    }
    let Some(target) = picking_targets.target(&PickingMode::Selectable) else {
        return;
    };
    let Some(entity) = target.entity else {
        return;
    };

    primary_selection.iter().for_each(|not_primary| {
        commands.entity(not_primary).remove::<PrimarySelection>();
    });

    let mut commands = commands.entity(entity);
    commands.insert(GizmoTarget::default());
    commands.insert(PrimarySelection);
}

fn pick_spawn_position(
    mut state: ResMut<EditorState>,
    mouse: Res<ButtonInput<MouseButton>>,
    picking_targets: Res<PickingTargets>,
    egui_has_pointer: Res<EguiHasPointer>,
) {
    if egui_has_pointer.0 {
        return;
    }
    if state.spawn.mode != SpawnPickerMode::Picking {
        return;
    }

    state.spawn.position = picking_targets
        .target(&PickingMode::Terrain)
        .as_ref()
        .map(|target| target.point + target.normal * 0.1);

    if mouse.just_released(MouseButton::Left) {
        state.spawn.mode = if state.spawn.position.is_some() {
            SpawnPickerMode::Spawning
        } else {
            SpawnPickerMode::Inactive
        };
    }
}

fn place_new_entity(
    time: Res<Time>,
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    egui_has_pointer: Res<EguiHasPointer>,
    picking_targets: Res<PickingTargets>,
    placing: Option<Single<(Entity, &mut Transform, &Placing)>>,
) {
    let Some(placing) = placing else {
        return;
    };

    let (entity, mut transform, placement) = placing.into_inner();
    let finish = mouse.just_released(MouseButton::Left)
        && !egui_has_pointer.0
        && (time.elapsed_secs_f64() - placement.spawned_time >= 0.75);

    if let Some(target) = picking_targets.targets(&placement.modes) {
        transform.translation = target.point + placement.offset;

        if placement.align_to_hit_normal {
            transform.look_at(target.point + target.normal, Vec3::Y);
            transform.rotate_local_x(-90.0_f32.to_radians());
        }

        if finish {
            let mut commands = commands.entity(entity);
            commands.remove::<Placing>();
            commands.insert(GizmoTarget::default());
        }
    } else if finish {
        commands.queue(CancelEntityPlacement);
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

//
// Utility
//

/// Adapted from: https://bevy-cheatbook.github.io/cookbook/cursor2world.html
pub fn cursor_to_ground_plane(
    window: &Window,
    (camera, camera_transform): (&Camera, &GlobalTransform),
) -> Option<Vec2> {
    let Some(cursor_position) = window.cursor_position() else {
        return None;
    };
    let plane_origin = Vec3::ZERO;
    let plane = InfinitePlane3d::new(Vec3::Y);
    let Ok(ray) = camera.viewport_to_world(camera_transform, cursor_position) else {
        return None;
    };
    let Some(distance) = ray.intersect_plane(plane_origin, plane) else {
        return None;
    };
    let global_cursor = ray.get_point(distance);

    Some(global_cursor.xz())
}
