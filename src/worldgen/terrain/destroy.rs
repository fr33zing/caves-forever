use std::sync::{Arc, Mutex};

use bevy::{prelude::*, tasks::AsyncComputeTaskPool, utils::HashSet};
use rayon::iter::ParallelIterator;

use crate::worldgen::chunk::ChunksAABB;

use super::{
    chunk_samples, merge_sdf_with_hardness, ChunkRemeshRequest, ChunkRemeshTask, ChunkSpawnRequest,
    ChunkSpawnTask, TerrainState, TerrainStateResource, VOXEL_REAL_SIZE,
};

#[derive(Event, Clone, Copy)]
pub struct DestroyTerrainEvent {
    pub position: Vec3,
    pub radius: f32,
    pub force: f32,
}

impl DestroyTerrainEvent {
    pub fn unevent(&self) -> DestroyTerrain {
        DestroyTerrain {
            position: self.position,
            radius: self.radius,
            force: self.force,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DestroyTerrain {
    pub position: Vec3,
    pub radius: f32,
    pub force: f32,
}

impl DestroyTerrain {
    fn world_extents(&self) -> (Vec3, Vec3) {
        let inflate = VOXEL_REAL_SIZE; // World units, not chunks
        let radius = Vec3::splat(self.radius + inflate);
        let min = self.position - radius;
        let max = self.position + radius;

        (min, max)
    }
}

pub struct DestroyTerrainParams {
    pub state: Arc<Mutex<TerrainState>>,
    pub destruction: Vec<DestroyTerrain>,
}

pub fn begin_destroy_terrain(
    mut event: EventReader<DestroyTerrainEvent>,
    spawn_tasks: Query<&ChunkSpawnTask>,
    remesh_tasks: Query<&ChunkRemeshTask>,
    state: Res<TerrainStateResource>,
) {
    // Wait until all other spawn/remesh tasks are finished
    {
        let state = state.lock().unwrap();
        if !spawn_tasks.is_empty()
            || !remesh_tasks.is_empty()
            || !state.spawn_requests.is_empty()
            || !state.remesh_requests.is_empty()
        {
            return;
        }
    }

    let destruction: Vec<DestroyTerrain> = event.read().map(|e| e.unevent()).collect();

    if destruction.len() == 0 {
        return;
    }

    let params = DestroyTerrainParams {
        state: state.clone(),
        destruction,
    };

    let task_pool = AsyncComputeTaskPool::get();
    task_pool
        .spawn(async move { destroy_terrain(params) })
        .detach();
}

fn destroy_terrain(params: DestroyTerrainParams) {
    let mut affected_chunks = HashSet::<IVec3>::new();
    let mut spawn_requests = Vec::<ChunkSpawnRequest>::new();
    let mut remesh_requests = Vec::<ChunkRemeshRequest>::new();

    params.destruction.iter().for_each(|event| {
        let aabb = ChunksAABB::from_world_aabb(event.world_extents(), 0);
        affected_chunks.extend(aabb.chunks.clone());
    });

    let mut state = params.state.lock().unwrap();

    for chunk_pos in affected_chunks {
        let Some((data, chunk_entity)) = state.chunk_data.get_mut(&chunk_pos) else {
            spawn_requests.push(ChunkSpawnRequest {
                chunk_pos,
                copy_borders: true,
                destruction: Some(params.destruction.clone()),
            });
            continue;
        };

        let world_pos = data.world_pos();
        for destroy in params.destruction.iter() {
            let changed = merge_sdf_with_hardness(data, destroy.force, || {
                chunk_samples(&world_pos)
                    .map(|point| point.distance(destroy.position) - destroy.radius)
                    .collect()
            });
            if changed {
                remesh_requests.push(ChunkRemeshRequest {
                    chunk_pos,
                    chunk_entity: *chunk_entity,
                });
            }
        }
    }

    state.spawn_requests.extend(spawn_requests);
    state.remesh_requests.extend(remesh_requests);
}
