use std::sync::{Arc, Mutex};

use avian3d::prelude::*;
use bevy::{
    math::Vec3A,
    prelude::*,
    render::primitives::Aabb,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use rayon::iter::{IndexedParallelIterator, IntoParallelRefMutIterator, ParallelIterator};

use super::{
    boundary::LoadingBoundary,
    change_detection::{TerrainSource, TerrainSourceArc},
    utility::*,
    CaveMaterialHandle, Chunk, ChunkData, ChunkRemeshRequest, DestroyTerrain, TerrainState,
    TerrainStateMutex, CHUNK_SAMPLE_RESOLUTION, CHUNK_SIZE_F,
};
use crate::{physics::GameLayer, tnua::IsPlayer, worldgen::voxel::VoxelMaterial};

#[derive(Default, Clone)]
pub struct ChunkSpawnRequest {
    pub chunk_pos: IVec3,
    pub copy_borders: bool,
    pub destruction: Option<Vec<DestroyTerrain>>,
}

#[derive(Default, Clone)]
struct ChunkSpawnParams {
    state: Arc<Mutex<TerrainState>>,
    request: ChunkSpawnRequest,
    source: Arc<TerrainSource>,
}

impl ChunkSpawnParams {
    pub fn new(state: Arc<Mutex<TerrainState>>) -> Self {
        Self { state, ..default() }
    }

    pub fn with_request(&self, spawn_chunk: &ChunkSpawnRequest) -> Self {
        let mut clone = self.clone();
        clone.request = spawn_chunk.clone();

        clone
    }
}

struct ChunkSpawnResult {
    data: ChunkData,
    mesh: Mesh,
    collider: Collider,
}

#[derive(Component)]
pub struct ChunkSpawnTask {
    task: Task<Option<ChunkSpawnResult>>,
    chunk_pos: IVec3,
    boundary: Entity,
}

pub fn begin_spawn_chunks(
    mut commands: Commands,
    state: Res<TerrainStateMutex>,
    source: Res<TerrainSourceArc>,
    player: Option<Single<&Transform, With<IsPlayer>>>,
    spawn_tasks: Query<&ChunkSpawnTask>,
) {
    let params = ChunkSpawnParams::new(state.clone());
    let mut state = state.lock().unwrap();

    if state.spawn_requests.is_empty() {
        return;
    }

    let mut max_tasks: usize = 128;
    if let Some(player) = player {
        let player_chunk = player.translation / CHUNK_SIZE_F;
        let player_chunk_ivec = player_chunk.as_ivec3();

        state
            .spawn_requests
            .sort_unstable_by_key(|a| a.chunk_pos.distance_squared(player_chunk_ivec));

        let chunks_from_closest = state.spawn_requests[0]
            .chunk_pos
            .as_vec3()
            .distance(player_chunk);

        if chunks_from_closest <= 2.0 {
            max_tasks = 4;
        }
    };
    let n = (max_tasks - spawn_tasks.iter().count()).clamp(0, state.spawn_requests.len());
    let requests = state.spawn_requests.drain(0..n);

    let task_pool = AsyncComputeTaskPool::get();
    requests.for_each(|request| {
        let mut params = params.with_request(&request);
        params.source = source.0.clone();

        let task = task_pool.spawn(async move { spawn_chunks(params) });
        let boundary = commands.spawn(LoadingBoundary::new(request.chunk_pos)).id();
        commands.spawn(ChunkSpawnTask {
            task,
            chunk_pos: request.chunk_pos,
            boundary,
        });
    });
}

pub fn receive_spawn_chunks(
    mut commands: Commands,
    state: Res<TerrainStateMutex>,
    mut meshes: ResMut<Assets<Mesh>>,
    material: Res<CaveMaterialHandle>,
    mut spawn_tasks: Query<(Entity, &mut ChunkSpawnTask)>,
) {
    for (task_entity, mut task) in spawn_tasks.iter_mut() {
        let status = block_on(future::poll_once(&mut task.task));

        let Some(result) = status else {
            continue;
        };

        let mut state = state.lock().unwrap();

        if let Some((_, entity)) = state.chunk_data.get(&task.chunk_pos) {
            commands.entity(*entity).clear();
        }

        if let Some(generated) = result {
            let scale = Vec3::splat(1.0 / CHUNK_SAMPLE_RESOLUTION);
            let half_extents = Vec3A::splat(CHUNK_SIZE_F / 2.0);
            let world_pos = generated.data.world_pos();

            let commands = commands.spawn((
                generated.collider,
                Chunk,
                Aabb {
                    center: half_extents,
                    half_extents,
                },
                Transform::from_translation(world_pos).with_scale(scale),
                RigidBody::Static,
                CollisionLayers::new(GameLayer::World, LayerMask::ALL),
                DebugRender::default().without_collider().without_axes(),
                Mesh3d(meshes.add(generated.mesh)),
                MeshMaterial3d(material.0.clone()),
            ));
            let entity = commands.id();

            state
                .chunk_data
                .insert(generated.data.chunk_pos, (generated.data, entity));
        }

        commands.entity(task.boundary).clear();
        commands.entity(task_entity).clear();
    }
}

fn spawn_chunks(params: ChunkSpawnParams) -> Option<ChunkSpawnResult> {
    let mut data = ChunkData::new(params.request.chunk_pos);
    let world_pos = data.world_pos();

    data.sdf
        .par_iter_mut()
        .zip(&mut data.materials)
        .enumerate()
        .for_each(|(i, (distance, material))| {
            let pos = delinearize_to_world_pos(world_pos, i as u32);

            // Sample brushes
            for brush in params.source.brushes.values() {
                let mut sample = brush.sample(pos);
                if sample.distance < *distance {
                    postprocess_sample(&mut sample);
                    *distance = sample.distance;
                    *material = sample.material;
                } else if material == &VoxelMaterial::Unset {
                    postprocess_sample(&mut sample);
                    *material = sample.material;
                }
            }

            // Apply material-specific noise
            *distance += material.sdf_noise(&pos, distance);
        });

    // Apply destruction
    if let Some(destruction) = params.request.destruction {
        for destroy in destruction.iter() {
            merge_sdf_with_hardness(&mut data, destroy.force, || {
                chunk_samples(&world_pos)
                    .map(|point| point.distance(destroy.position) - destroy.radius)
                    .collect()
            });
        }
    }

    // Copy borders
    if params.request.copy_borders {
        let mut state = params.state.lock().unwrap();
        let mut remesh_requests = Vec::<ChunkRemeshRequest>::new();
        let neighbors = state.neighbors(&params.request.chunk_pos);

        for neighbor in neighbors {
            let Some((neighbor, entity)) = state.chunk_data.get_mut(&neighbor) else {
                continue;
            };

            // Copy borders FROM adjacent chunks
            copy_borders(&mut data, neighbor);

            // Copy borders TO adjacent chunks
            let changed = copy_borders(neighbor, &data);
            if changed {
                remesh_requests.push(ChunkRemeshRequest {
                    chunk_pos: neighbor.chunk_pos,
                    chunk_entity: *entity,
                });
            }
        }

        state.remesh_requests.extend(remesh_requests);
    }

    let Some((mesh, collider)) = mesh_chunk(&data) else {
        return None;
    };

    Some(ChunkSpawnResult {
        data,
        mesh,
        collider,
    })
}
