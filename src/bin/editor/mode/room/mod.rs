use std::collections::HashSet;

use bevy::{
    asset::{Assets, RenderAssetUsages},
    prelude::{Changed, Commands, Component, Entity, Mesh, Mesh3d, Query, Res, ResMut, Transform},
    render::mesh::{Indices, PrimitiveTopology},
    time::Time,
};
use uuid::Uuid;

use crate::state::{EditorState, FilePayload};
use mines::worldgen::{
    asset::{RoomPartPayload, RoomPartUuid},
    brush::TerrainBrush,
};

pub mod ui;
mod utility;

use utility::room_part_to_editor_bundle;

#[derive(Component)]
pub struct UpdatePreviewBrush {
    time: f64,
    uuid: Uuid,
}

//
// Systems
//

// Hook: update
pub fn detect_additions(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    state: Res<EditorState>,
    parts: Query<&RoomPartUuid>,
) {
    let Some(data) = state.files.current_data() else {
        return;
    };
    let FilePayload::Room(data) = data else {
        return;
    };

    let mut existing = HashSet::<Uuid>::new();
    parts.iter().for_each(|uuid| {
        existing.insert(uuid.0);
    });
    data.parts.iter().for_each(|(uuid, part)| {
        if !existing.contains(uuid) {
            commands.spawn(room_part_to_editor_bundle(part, &mut meshes));
        }
    });
}

// Hook: update
pub fn detect_world_changes(
    time: Res<Time>,
    mut commands: Commands,
    mut state: ResMut<EditorState>,
    parts: Query<(&Transform, &RoomPartUuid), Changed<Transform>>,
    update_preview_brushes: Query<(Entity, &UpdatePreviewBrush)>,
) {
    let Some(data) = state.files.current_data_mut() else {
        return;
    };
    let FilePayload::Room(data) = data else {
        return;
    };

    let mut update_uuids = Vec::<Uuid>::new();

    parts.iter().for_each(|(transform, uuid)| {
        let Some(part) = data.parts.get_mut(&uuid.0) else {
            return;
        };

        part.transform = *transform;
        update_uuids.push(uuid.0);
    });

    for (entity, update_preview_brush) in update_preview_brushes.into_iter() {
        if update_uuids.contains(&update_preview_brush.uuid) {
            commands.entity(entity).clear();
        }
    }
    for uuid in update_uuids {
        commands.spawn(UpdatePreviewBrush {
            time: time.elapsed_secs_f64(),
            uuid,
        });
    }
}

pub fn detect_hash_changes(
    time: Res<Time>,
    state: Res<EditorState>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut parts: Query<(Entity, &mut RoomPartUuid)>,
    update_preview_brushes: Query<(Entity, &UpdatePreviewBrush)>,
) {
    let Some(data) = state.files.current_data() else {
        return;
    };
    let FilePayload::Room(data) = data else {
        return;
    };

    let mut update_uuids = Vec::<Uuid>::new();

    parts.iter_mut().for_each(|mut world_part| {
        let (entity, ref mut uuid_hash) = world_part;
        let (ref uuid, ref mut world_hash) = (uuid_hash.0, uuid_hash.1);

        let Some(data_part) = data.parts.get(uuid) else {
            todo!();
        };

        match data_part.data {
            RoomPartPayload::Stl {
                geometry_hash,
                ref vertices,
                ref indices,
                ..
            } => {
                if *world_hash == Some(geometry_hash) {
                    return;
                }

                world_part.1 .1 = Some(geometry_hash);
                let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
                    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone())
                    .with_inserted_indices(Indices::U32(indices.clone()));
                commands.entity(entity).insert(Mesh3d(meshes.add(mesh)));
                update_uuids.push(*uuid);
            }
        }
    });

    for (entity, update_preview_brush) in update_preview_brushes.into_iter() {
        if update_uuids.contains(&update_preview_brush.uuid) {
            commands.entity(entity).clear();
        }
    }
    for uuid in update_uuids {
        commands.spawn(UpdatePreviewBrush {
            time: time.elapsed_secs_f64(),
            uuid,
        });
    }
}

// Hook: update
pub fn update_preview_brushes(
    mut commands: Commands,
    time: Res<Time>,
    state: Res<EditorState>,
    update_preview_brushes: Query<(Entity, &UpdatePreviewBrush)>,
    terrain_brushes: Query<(Entity, &TerrainBrush)>,
) {
    let Some(data) = state.files.current_data() else {
        return;
    };
    let FilePayload::Room(data) = data else {
        return;
    };

    const TIMER_SECS: f64 = 0.5;

    let mut clear_brushes = Vec::<Uuid>::new();

    update_preview_brushes.iter().for_each(|(upb_entity, upb)| {
        if time.elapsed_secs_f64() - upb.time < TIMER_SECS {
            return;
        }

        let Some(part) = data.parts.get(&upb.uuid) else {
            todo!();
        };

        clear_brushes.push(upb.uuid);
        commands.entity(upb_entity).clear();
        commands.spawn(part.to_brush_request());
    });

    clear_brushes.into_iter().for_each(|uuid| {
        let uuid = uuid.to_string();

        terrain_brushes.iter().for_each(|(entity, brush)| {
            if brush.uuid() == &uuid {
                commands.entity(entity).despawn();
            }
        });
    });
}
