use core::f32;
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
};
use nalgebra::Vector3;

use crate::{
    camera,
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
    pub prev_mode: Option<EditorMode>,
    pub prev_view: Option<EditorViewMode>,
    pub mode_systems: HashMap<EditorMode, ModeSystems>,
    pub cleanup_system: SystemId,
    pub camera_on_change_mode_system: SystemId,
    pub update_files_changed_status_system: SystemId,
    pub playtest_system: SystemId,
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
        let camera_on_change_mode_system = world.register_system(camera::on_change_mode);
        let cleanup_system = world.register_system(cleanup);
        let update_files_changed_status_system = world.register_system(update_files_changed_status);
        let playtest_system = world.register_system(playtest);

        app.insert_resource(ModeSwitcher {
            prev_mode: default(),
            prev_view: default(),
            mode_systems: default(),
            cleanup_system,
            camera_on_change_mode_system,
            update_files_changed_status_system,
            playtest_system,
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
                    world.register_system(room::detect_hash_changes),
                    world.register_system(room::update_preview_brushes),
                ],
                ..default()
            },
        );
    });
}

pub fn cleanup(
    mut commands: Commands,
    state: Res<EditorState>,
    mode_specific_entities: Query<(Entity, &ModeSpecific)>,
) {
    mode_specific_entities
        .iter()
        .for_each(|(entity, ModeSpecific(mode, view))| {
            let mut remove = false;
            if *mode != state.mode() {
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

fn switch_modes(world: &mut World) {
    let (curr_mode, curr_view) =
        world.resource_scope(|_, state: Mut<EditorState>| (state.mode(), state.view));

    let systems: Vec<SystemId> = world.resource_scope(|_, mut switcher: Mut<ModeSwitcher>| {
        let mut systems = Vec::<Option<SystemId>>::new();
        let prev_mode = switcher.prev_mode;
        let changed_mode = switcher.prev_mode != Some(curr_mode);
        let changed_view = switcher.prev_view != Some(curr_view);

        if changed_mode {
            if let Some(prev_mode) = prev_mode {
                if let Some(prev_systems) = switcher.mode_systems.get(&prev_mode) {
                    systems.push(prev_systems.exit);
                }
            }

            if let Some(curr_systems) = switcher.mode_systems.get(&curr_mode) {
                systems.push(curr_systems.enter);
            }

            switcher.prev_mode = Some(curr_mode);
        }

        if changed_view {
            if let Some(curr_systems) = switcher.mode_systems.get(&curr_mode) {
                systems.push(curr_systems.enter_view.get(&curr_view).copied());
            }

            switcher.prev_view = Some(curr_view);
        }

        if changed_mode || changed_view {
            systems.push(Some(switcher.camera_on_change_mode_system));
            systems.push(Some(switcher.cleanup_system));
        }

        systems.push(Some(switcher.update_files_changed_status_system));
        systems.push(Some(switcher.playtest_system));

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

    let Some(single) = camera else {
        return;
    };

    let (mut camera, mut trackball, mut light) = single.into_inner();
    let mut queue = CommandQueue::default();

    match next_mode {
        SpawnPickerMode::Inactive => {
            camera.is_active = true;
            light.range = 2048.0;
            queue.push(DespawnPlayerCommand);
            state.spawn.position = None;

            // Camera doesn't switch properly unless we change the frame.
            trackball
                .frame
                .local_slide(&Vector3::new(0.0, f32::EPSILON, 0.0));
        }
        SpawnPickerMode::Playing => {
            camera.is_active = false;
            light.range = 0.0;
            queue.push(SpawnPlayerCommand {
                position: spawn_pos + Vec3::Y * PLAYER_HEIGHT / 2.0,
            });
        }
        _ => {}
    };

    state.spawn.mode = next_mode;
    commands.append(&mut queue);
}
