use bevy::{pbr::wireframe::WireframeColor, picking::backend::ray::RayMap, prelude::*};
use transform_gizmo_bevy::GizmoTarget;

use crate::{
    state::{EditorState, SpawnPickerMode},
    ui::CursorOverEguiPanel,
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
pub struct Selectable;

#[derive(Component)]
pub struct PrimarySelection;

pub struct PickingPlugin;

impl Plugin for PickingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(MeshPickingPlugin);
        app.insert_resource(MeshPickingSettings {
            require_markers: true,
            ray_cast_visibility: RayCastVisibility::VisibleInView,
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
