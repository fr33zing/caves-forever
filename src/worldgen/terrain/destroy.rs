use std::sync::{Arc, Mutex};

use bevy::{prelude::*, tasks::AsyncComputeTaskPool, utils::HashSet};
use rayon::iter::ParallelIterator;

use crate::worldgen::chunk::ChunksAABB;

use super::{
    chunk_samples, merge_sdf_with_hardness, SpawnChunkRequest, TerrainState, TerrainStateResource,
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

#[derive(Clone, Copy)]
pub struct DestroyTerrain {
    pub position: Vec3,
    pub radius: f32,
    pub force: f32,
}

impl DestroyTerrain {
    fn world_extents(&self) -> (Vec3, Vec3) {
        let inflate = 1.0; // World units, not chunks
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
    state: Res<TerrainStateResource>,
) {
    let task_pool = AsyncComputeTaskPool::get();

    let destruction: Vec<DestroyTerrain> = event.read().map(|e| e.unevent()).collect();

    if destruction.len() == 0 {
        return;
    }

    let params = DestroyTerrainParams {
        state: state.clone(),
        destruction,
    };

    task_pool
        .spawn(async move { destroy_terrain(params) })
        .detach();
}

fn destroy_terrain(params: DestroyTerrainParams) {
    let mut affected_chunks = HashSet::<IVec3>::new();
    let mut chunks_to_generate = HashSet::<IVec3>::new();
    let mut chunks_to_remesh = HashSet::<(IVec3, Entity)>::new();

    params.destruction.iter().for_each(|event| {
        let aabb = ChunksAABB::from_world_aabb(event.world_extents(), 0);
        affected_chunks.extend(aabb.chunks.clone());
    });

    let state = params.state.clone();
    let mut state = state.lock().unwrap();

    for chunk in affected_chunks {
        let Some((data, entity)) = state.chunk_data.get_mut(&chunk) else {
            chunks_to_generate.insert(chunk);
            continue;
        };

        chunks_to_remesh.insert((data.chunk_pos, *entity));

        let world_pos = data.world_pos();
        for destroy in params.destruction.iter() {
            let changed = merge_sdf_with_hardness(data, destroy.force, || {
                chunk_samples(&world_pos)
                    .map(|point| point.distance(destroy.position) - destroy.radius)
                    .collect()
            });
            if changed {
                chunks_to_remesh.insert((data.chunk_pos, *entity));
            }
        }
    }

    state.chunks_to_remesh.extend(chunks_to_remesh);

    for chunk_pos in chunks_to_generate {
        state.chunks_to_spawn.push(SpawnChunkRequest {
            chunk_pos,
            copy_borders: true,
            destruction: Some(params.destruction.clone()),
        });
    }
}
