use std::collections::HashMap;

use bevy::{ecs::system::SystemId, prelude::*};

use crate::{
    camera,
    state::{EditorMode, EditorState, EditorViewMode},
};

pub mod tunnels;

#[derive(Default, Clone)]
struct ModeSystems {
    exit: Option<SystemId>,
    enter: Option<SystemId>,
    //exit_view: HashMap<EditorViewMode, SystemId>,
    enter_view: HashMap<EditorViewMode, SystemId>,
    update: Vec<SystemId>,
}

#[derive(Resource)]
struct ModeSwitcher {
    pub prev_mode: Option<EditorMode>,
    pub prev_view: Option<EditorViewMode>,
    pub mode_systems: HashMap<EditorMode, ModeSystems>,
    pub cleanup: SystemId,
    pub camera_on_change_mode_system: SystemId,
}

#[derive(Component)]
pub struct ModeSpecific(pub EditorMode, pub Option<EditorViewMode>);

/// These gizmos will render above the rest.
#[derive(Default, Reflect, GizmoConfigGroup)]
pub struct EditorHandleGizmos;

pub struct EditorModesPlugin;

impl Plugin for EditorModesPlugin {
    fn build(&self, app: &mut App) {
        app.init_gizmo_group::<EditorHandleGizmos>();

        let world = app.world_mut();
        let camera_on_change_mode_system = world.register_system(camera::on_change_mode);
        let cleanup = world.register_system(cleanup);

        app.insert_resource(ModeSwitcher {
            prev_mode: default(),
            prev_view: default(),
            mode_systems: default(),
            cleanup,
            camera_on_change_mode_system,
        });

        app.add_systems(Startup, (camera::setup, setup).chain());
        app.add_systems(Update, (switch_modes, update_curr_mode).chain());
    }
}

pub fn setup(world: &mut World) {
    world.resource_scope(|_, mut gizmos_config: Mut<GizmoConfigStore>| {
        gizmos_config
            .config_mut::<EditorHandleGizmos>()
            .0
            .depth_bias = -1.0;
    });

    world.resource_scope(|world, mut switcher: Mut<ModeSwitcher>| {
        switcher.mode_systems.insert(
            EditorMode::Tunnels,
            ModeSystems {
                enter: Some(world.register_system(tunnels::spawn_size_reference_labels)),
                update: vec![
                    world.register_system(tunnels::pick_profile_point),
                    world.register_system(tunnels::drag_profile_point),
                    world.register_system(tunnels::update_tunnel_info),
                    world.register_system(tunnels::draw_size_references),
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
                commands.entity(entity).clear();
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
            systems.push(Some(switcher.cleanup));
        }

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
