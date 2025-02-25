use std::collections::HashMap;

use bevy::{
    ecs::{
        system::{SystemId, SystemState},
        world::CommandQueue,
    },
    prelude::*,
    render::view::RenderLayers,
};
use bevy_trackball::TrackballCamera;
use common_macros::hash_map;
use lib::{
    player::{consts::PLAYER_HEIGHT, DespawnPlayerCommand, SpawnPlayerCommand},
    render_layer,
    worldgen::brush::TerrainBrush,
};
use nalgebra::Vector3;

use crate::{
    camera,
    picking::CancelEntityPlacement,
    state::{EditorMode, EditorState, EditorViewMode, SpawnPickerMode},
};

pub mod room;
pub mod tunnel;

/// This command must be executed after a file is reverted.
/// It ensures that the visual representation of the file is reset.
pub struct RevertCommand;
impl Command for RevertCommand {
    fn apply(self, world: &mut World) {
        let mut systems_to_run = Vec::<Option<SystemId>>::new();
        {
            let mut system_state: SystemState<(
                Commands,
                Res<EditorState>,
                Res<ModeSwitcher>,
                Query<Entity, With<ModeSpecific>>,
            )> = SystemState::new(world);
            let (mut commands, state, switcher, mode_specific_entities) =
                system_state.get_mut(world);

            mode_specific_entities.iter().for_each(|entity| {
                commands.entity(entity).clear();
            });

            let (mode, view) = (state.mode(), state.view);
            let Some(mode) = mode else {
                return;
            };
            if let Some(systems) = switcher.mode_systems.get(&mode) {
                systems_to_run = vec![
                    systems.exit,
                    systems.enter,
                    systems.enter_view.get(&view).copied(),
                ];
            }

            system_state.apply(world);
        }

        systems_to_run.iter().for_each(|system| {
            if let Some(system) = system {
                world.run_system(*system).unwrap();
            }
        });
    }
}

#[derive(Default, Clone)]
struct ModeSystems {
    exit: Option<SystemId>,
    enter: Option<SystemId>,
    enter_view: HashMap<EditorViewMode, SystemId>,
    update: Vec<SystemId>,
}

#[derive(Resource)]
struct ModeSwitcher {
    pub prev_file: Option<usize>,
    pub prev_mode: Option<EditorMode>,
    pub prev_view: Option<EditorViewMode>,
    pub mode_systems: HashMap<EditorMode, ModeSystems>,
    pub cleanup_mode_specific_entities: SystemId,
    pub cleanup_terrain: SystemId,
    pub cancel_placement_and_playtest: SystemId,
    pub camera_on_change_mode: SystemId,
    pub update_files_changed_status: SystemId,
    pub playtest: SystemId,
}

#[derive(Component)]
pub struct ModeSpecific(pub EditorMode, pub Option<EditorViewMode>);

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct EditorGizmos;

#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct EditorPreviewGizmos;

pub struct EditorModesPlugin;

impl Plugin for EditorModesPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<EditorGizmos>();
        app.init_gizmo_group::<EditorPreviewGizmos>();

        let world = app.world_mut();
        let camera_on_change_mode = world.register_system(camera::on_change_mode);
        let cleanup_mode_specific_entities = world.register_system(cleanup_mode_specific_entities);
        let cleanup_terrain = world.register_system(cleanup_terrain);
        let cancel_placement_and_playtest = world.register_system(cancel_placement_and_playtest);
        let update_files_changed_status = world.register_system(update_files_changed_status);
        let playtest = world.register_system(playtest);

        app.insert_resource(ModeSwitcher {
            prev_file: default(),
            prev_mode: default(),
            prev_view: default(),
            mode_systems: default(),
            cleanup_mode_specific_entities,
            cleanup_terrain,
            cancel_placement_and_playtest,
            camera_on_change_mode,
            update_files_changed_status,
            playtest,
        });

        app.add_systems(Startup, (camera::setup, setup).chain());
        app.add_systems(Update, (switch_modes, update_curr_mode).chain());
    }
}

pub fn setup(world: &mut World) {
    world.resource_scope(|_, mut gizmos_config: Mut<GizmoConfigStore>| {
        gizmos_config.config_mut::<EditorGizmos>().0.render_layers =
            RenderLayers::layer(render_layer::EDITOR);
        gizmos_config
            .config_mut::<EditorPreviewGizmos>()
            .0
            .render_layers = RenderLayers::layer(render_layer::EDITOR_PREVIEW);
    });

    world.resource_scope(|world, mut switcher: Mut<ModeSwitcher>| {
        switcher.mode_systems.insert(
            EditorMode::Tunnels,
            ModeSystems {
                enter: Some(world.register_system(tunnel::spawn_size_reference_labels)),
                enter_view: hash_map! {
                    EditorViewMode::Preview => world.register_system(tunnel::enter_preview)
                },
                update: vec![
                    world.register_system(tunnel::pick_profile_point),
                    world.register_system(tunnel::drag_profile_point),
                    world.register_system(tunnel::update_tunnel_info),
                    world.register_system(tunnel::draw_size_references),
                    world.register_system(tunnel::remesh_preview_path),
                    world.register_system(tunnel::update_preview_brush),
                ],
                ..default()
            },
        );

        switcher.mode_systems.insert(
            EditorMode::Rooms,
            ModeSystems {
                update: vec![
                    world.register_system(room::detect_world_changes),
                    world.register_system(room::detect_additions),
                    world.register_system(room::detect_removals),
                    world.register_system(room::detect_hash_changes),
                    world.register_system(room::update_preview_brushes),
                    world.register_system(room::correct_portal_orientations),
                ],
                ..default()
            },
        );
    });
}

pub fn cleanup_mode_specific_entities(
    mut commands: Commands,
    state: Res<EditorState>,
    mode_specific_entities: Query<(Entity, &ModeSpecific)>,
) {
    mode_specific_entities
        .iter()
        .for_each(|(entity, ModeSpecific(mode, view))| {
            let mut remove = false;
            if Some(*mode) != state.mode() {
                remove = true;
            } else {
                if let Some(view) = view {
                    remove = *view != state.view;
                }
            }
            if remove {
                commands.entity(entity).despawn_recursive();
            }
        });
}

pub fn cleanup_terrain(mut commands: Commands, terrain_brushes: Query<Entity, With<TerrainBrush>>) {
    terrain_brushes.iter().for_each(|brush| {
        commands.entity(brush).clear();
    });
}

fn switch_modes(world: &mut World) {
    let (curr_file, curr_mode, curr_view) = world.resource_scope(|_, state: Mut<EditorState>| {
        (state.files.current, state.mode(), state.view)
    });

    let systems: Vec<SystemId> = world.resource_scope(|_, mut switcher: Mut<ModeSwitcher>| {
        let mut systems = Vec::<Option<SystemId>>::new();
        let prev_mode = switcher.prev_mode;
        let changed_file = switcher.prev_file != curr_file;
        let changed_mode = switcher.prev_mode != curr_mode;
        let changed_view = switcher.prev_view != Some(curr_view);

        if changed_file {
            systems.push(Some(switcher.cleanup_terrain));

            switcher.prev_file = curr_file;
        }

        if changed_mode {
            if let Some(prev_mode) = prev_mode {
                if let Some(prev_systems) = switcher.mode_systems.get(&prev_mode) {
                    systems.push(prev_systems.exit);
                }
            }

            if let Some(curr_mode) = curr_mode {
                if let Some(curr_systems) = switcher.mode_systems.get(&curr_mode) {
                    systems.push(curr_systems.enter);
                }
            }

            switcher.prev_mode = curr_mode;
        }

        if changed_view {
            if let Some(curr_mode) = curr_mode {
                if let Some(curr_systems) = switcher.mode_systems.get(&curr_mode) {
                    systems.push(curr_systems.enter_view.get(&curr_view).copied());
                }
            }

            switcher.prev_view = Some(curr_view);
        }

        if changed_mode || changed_view {
            systems.push(Some(switcher.camera_on_change_mode));
            systems.push(Some(switcher.cleanup_mode_specific_entities));
        }

        if changed_file || changed_mode || changed_view {
            systems.push(Some(switcher.cancel_placement_and_playtest));
        }

        systems.push(Some(switcher.update_files_changed_status));
        systems.push(Some(switcher.playtest));

        systems
            .into_iter()
            .filter_map(|s| s.map(|s| s.clone()))
            .collect()
    });

    systems.into_iter().for_each(|system| {
        world.run_system(system).unwrap();
    });
}

fn update_curr_mode(world: &mut World) {
    let curr_mode = world.resource_scope(|_, state: Mut<EditorState>| state.mode());
    world.resource_scope(|world, switcher: Mut<ModeSwitcher>| {
        let Some(curr_mode) = curr_mode else {
            return;
        };
        let Some(curr_systems) = switcher.mode_systems.get(&curr_mode) else {
            return;
        };

        curr_systems
            .update
            .iter()
            .for_each(|s| world.run_system(s.clone()).unwrap());
    });
}

fn update_files_changed_status(world: &mut World) {
    world.resource_scope(|_, mut state: Mut<EditorState>| {
        state
            .files
            .files
            .iter_mut()
            .for_each(|f| f.changed = f.data != f.last_saved_data);
    });
}

fn cancel_placement_and_playtest(
    mut commands: Commands,
    mut state: ResMut<EditorState>,
    camera: Option<Single<(&mut Camera, &mut TrackballCamera, &mut PointLight)>>,
) {
    commands.queue(CancelEntityPlacement);

    let Some(camera) = camera else {
        return;
    };
    let (mut camera, mut trackball, mut light) = camera.into_inner();

    commands.queue(DespawnPlayerCommand);
    camera.is_active = true;
    light.range = 2048.0;
    state.spawn.mode = SpawnPickerMode::Inactive;
    state.spawn.position = None;

    // Camera doesn't switch properly unless we change the frame.
    trackball.frame.local_slide(&Vector3::new(0.0, 0.01, 0.0));
}

fn playtest(
    mut commands: Commands,
    mut state: ResMut<EditorState>,
    camera: Option<Single<(&mut Camera, &mut TrackballCamera, &mut PointLight)>>,
) {
    let Some(spawn_pos) = state.spawn.position else {
        return;
    };

    let next_mode: Option<SpawnPickerMode> = match state.spawn.mode {
        SpawnPickerMode::Spawning => Some(SpawnPickerMode::Playing),
        SpawnPickerMode::Despawning => Some(SpawnPickerMode::Inactive),
        _ => None,
    };

    let Some(next_mode) = next_mode else {
        return;
    };
    let Some(camera) = camera else {
        return;
    };

    let (mut camera, mut trackball, mut light) = camera.into_inner();
    let mut queue = CommandQueue::default();

    match next_mode {
        SpawnPickerMode::Inactive => {
            camera.is_active = true;
            light.range = 2048.0;
            queue.push(DespawnPlayerCommand);
            state.spawn.position = None;

            // Camera doesn't switch properly unless we change the frame.
            trackball.frame.local_slide(&Vector3::new(0.0, 0.01, 0.0));
        }
        SpawnPickerMode::Playing => {
            camera.is_active = false;
            light.range = 0.0;
            queue.push(SpawnPlayerCommand {
                position: Some(spawn_pos + Vec3::Y * PLAYER_HEIGHT / 2.0),
            });
        }
        _ => {}
    };

    state.spawn.mode = next_mode;
    commands.append(&mut queue);
}
