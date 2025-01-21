use bevy::{ecs::system::SystemId, prelude::*, utils::HashMap};

use crate::{
    camera,
    state::{EditorMode, EditorState, EditorViewMode},
};

mod tunnels;

#[derive(Default, Clone)]
struct ModeSystems {
    exit: Option<SystemId>,
    enter: Option<SystemId>,
    change_view: Option<SystemId>,
    update: Vec<SystemId>,
}

#[derive(Resource)]
struct ModeSwitcher {
    pub prev_mode: Option<EditorMode>,
    pub prev_view: Option<EditorViewMode>,
    pub mode_systems: HashMap<EditorMode, ModeSystems>,
    pub camera_on_change_mode_system: SystemId,
}

pub struct EditorModesPlugin;

impl Plugin for EditorModesPlugin {
    fn build(&self, app: &mut App) {
        let world = app.world_mut();
        let camera_on_change_mode_system = world.register_system(camera::on_change_mode);

        app.insert_resource(ModeSwitcher {
            prev_mode: default(),
            prev_view: default(),
            mode_systems: default(),
            camera_on_change_mode_system,
        });

        app.add_systems(Startup, (camera::setup, setup).chain());
        app.add_systems(Update, (switch_modes, update_curr_mode).chain());
    }
}

pub fn setup(world: &mut World) {
    world.resource_scope(|world, mut switcher: Mut<ModeSwitcher>| {
        switcher.mode_systems.insert(
            EditorMode::Tunnels,
            ModeSystems {
                exit: Some(world.register_system(tunnels::exit)),
                enter: Some(world.register_system(tunnels::enter)),
                update: vec![world.register_system(tunnels::update)],
                ..default()
            },
        );
    });
}

fn switch_modes(world: &mut World) {
    let (curr_mode, curr_view) =
        world.resource_scope(|_, state: Mut<EditorState>| (state.mode, state.view));

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
                systems.push(curr_systems.change_view);
            }

            switcher.prev_view = Some(curr_view);
        }

        if changed_mode || changed_view {
            systems.push(Some(switcher.camera_on_change_mode_system));
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
    let curr_mode = world.resource_scope(|_, state: Mut<EditorState>| state.mode);
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
