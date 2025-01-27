use std::sync::Arc;

use bevy::{prelude::*, utils::HashMap};

use crate::worldgen::{brush::TerrainBrush, chunk::ChunksAABB};

use super::{ChunkRemeshRequest, ChunkSpawnRequest, TerrainStateMutex};

#[derive(Default, Clone)]
pub struct TerrainSource {
    pub brushes: HashMap<Entity, TerrainBrush>,
}

#[derive(Resource, Default)]
pub struct TerrainSourceArc(pub Arc<TerrainSource>);

#[derive(Resource, Default)]
pub struct TerrainSourceChanges(pub Vec<ChunksAABB>);

pub struct TerrainChangeDetectionPlugin;

impl Plugin for TerrainChangeDetectionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainSourceArc>();
        app.init_resource::<TerrainSourceChanges>();
        app.add_systems(
            Update,
            (
                detect_brush_additions,
                detect_brush_removals,
                handle_chunk_changes,
            )
                .chain(),
        );
    }
}

fn detect_brush_additions(
    mut commands: Commands,
    sources: Res<TerrainSourceArc>,
    mut changed_aabbs: ResMut<TerrainSourceChanges>,
    changed_brushes: Query<(Entity, Ref<TerrainBrush>)>,
) {
    let mut additions: Vec<(Entity, TerrainBrush)> = Vec::new();

    changed_brushes.iter().for_each(|(entity, brush)| {
        if brush.is_added() {
            additions.push((entity, brush.clone()));
        }
    });

    if additions.is_empty() {
        return;
    }

    let mut sources = Arc::unwrap_or_clone(sources.0.clone());

    additions.into_iter().for_each(|(entity, brush)| {
        changed_aabbs.0.push(brush.chunks().clone());
        sources.brushes.insert(entity, brush);
    });

    commands.insert_resource(TerrainSourceArc(Arc::new(sources)));
}

fn detect_brush_removals(
    mut commands: Commands,
    sources: Res<TerrainSourceArc>,
    mut changed_aabbs: ResMut<TerrainSourceChanges>,
    mut removed_brushes: RemovedComponents<TerrainBrush>,
) {
    let mut removals: Vec<Entity> = Vec::new();

    removed_brushes.read().for_each(|entity| {
        removals.push(entity);
    });

    if removals.is_empty() {
        return;
    }

    let mut sources = Arc::unwrap_or_clone(sources.0.clone());

    removals.into_iter().for_each(|entity| {
        if let Some(brush) = sources.brushes.remove(&entity) {
            changed_aabbs.0.push(brush.chunks().clone());
        }
    });

    commands.insert_resource(TerrainSourceArc(Arc::new(sources)));
}

fn handle_chunk_changes(
    terrain_state: Res<TerrainStateMutex>,
    mut changed_aabbs: ResMut<TerrainSourceChanges>,
) {
    if changed_aabbs.0.len() == 0 {
        return;
    }

    let mut spawn = HashMap::<IVec3, ChunkSpawnRequest>::new();

    let changed_aabbs = std::mem::take(&mut changed_aabbs.0);
    let mut terrain_state = terrain_state.lock().unwrap();

    for aabb in changed_aabbs {
        for chunk_pos in aabb.chunks {
            if spawn.contains_key(&chunk_pos) {
                continue;
            }
            spawn.insert(
                chunk_pos,
                ChunkSpawnRequest {
                    chunk_pos,
                    copy_borders: false,
                    destruction: None,
                },
            );
        }
    }

    terrain_state.spawn_requests.extend(spawn.into_values());
}
