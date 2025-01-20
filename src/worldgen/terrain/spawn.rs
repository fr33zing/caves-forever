use std::sync::{Arc, Mutex};

use avian3d::prelude::*;
use bevy::{
    math::Vec3A,
    pbr::{ExtendedMaterial, OpaqueRendererMethod},
    prelude::*,
    render::primitives::Aabb,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use rayon::iter::ParallelIterator;

use crate::{
    materials::CaveMaterialExtension,
    physics::GameLayer,
    worldgen::brush::{collider::ColliderBrush, curve::CurveBrush, sweep::SweepBrush, Sampler},
};

use super::{
    boundary::LoadingBoundary, utility::*, Chunk, ChunkData, ChunkRemeshRequest, DestroyTerrain,
    TerrainState, TerrainStateResource, CHUNK_SAMPLE_RESOLUTION, CHUNK_SIZE_F,
};

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
    curves: Vec<CurveBrush>,
    sweeps: Vec<SweepBrush>,
    colliders: Vec<ColliderBrush>,
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
pub struct ChunkSpawnTask(Task<Option<ChunkSpawnResult>>, Entity);

pub fn begin_spawn_chunks(
    mut commands: Commands,
    state: Res<TerrainStateResource>,
    curve_brush_query: Query<&CurveBrush>,
    sweep_brush_query: Query<&SweepBrush>,
    collider_brush_query: Query<&ColliderBrush>,
) {
    let mut params = ChunkSpawnParams::new(state.clone());
    let mut state = state.lock().unwrap();

    if state.spawn_requests.is_empty() {
        return;
    }

    curve_brush_query.iter().for_each(|brush| {
        params.curves.push(brush.clone());
    });
    sweep_brush_query.iter().for_each(|brush| {
        params.sweeps.push(brush.clone());
    });
    collider_brush_query.iter().for_each(|brush| {
        params.colliders.push(brush.clone());
    });

    let task_pool = AsyncComputeTaskPool::get();
    state.spawn_requests.iter().for_each(|request| {
        let params = params.with_request(&request);
        let task = task_pool.spawn(async move { spawn_chunks(params) });

        let boundary = commands.spawn(LoadingBoundary::new(request.chunk_pos)).id();
        commands.spawn(ChunkSpawnTask(task, boundary));
    });

    state.spawn_requests.clear();
}

pub fn receive_spawn_chunks(
    mut commands: Commands,
    state: Res<TerrainStateResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, CaveMaterialExtension>>>,
    mut spawn_tasks: Query<(Entity, &mut ChunkSpawnTask)>,
) {
    for (task_entity, mut task) in spawn_tasks.iter_mut() {
        let status = block_on(future::poll_once(&mut task.0));

        let Some(result) = status else {
            continue;
        };

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
                MeshMaterial3d(materials.add(ExtendedMaterial {
                    base: StandardMaterial {
                        base_color: Color::srgb(0.5, 0.5, 0.5),
                        opaque_render_method: OpaqueRendererMethod::Auto,
                        ..Default::default()
                    },
                    extension: CaveMaterialExtension::new(7.0, 5.0),
                })),
            ));
            let entity = commands.id();

            let mut state = state.lock().unwrap();
            state
                .chunk_data
                .insert(generated.data.chunk_pos, (generated.data, entity));
        }

        commands.entity(task.1).clear();
        commands.entity(task_entity).despawn();
    }
}

fn spawn_chunks(params: ChunkSpawnParams) -> Option<ChunkSpawnResult> {
    let mut data = ChunkData::new(params.request.chunk_pos);
    let world_pos = data.world_pos();

    // Don't spawn chunks where they already exist
    if cfg!(debug_assertions) {
        let mut state = params.state.lock().unwrap();

        for (_, (other, _)) in state.chunk_data.iter_mut() {
            if other.chunk_pos == params.request.chunk_pos {
                panic!("tried to spawn chunk where one already exists");
            }
        }
    }

    // Sample curve brushes
    for brush in params.curves {
        merge_chunk(&mut data, || {
            chunk_samples(&world_pos)
                .map(|point| brush.sample(point))
                .collect()
        });
    }

    // Sample sweep brushes
    for brush in params.sweeps {
        merge_chunk(&mut data, || {
            chunk_samples(&world_pos)
                .map(|point| brush.sample(point))
                .collect()
        });
    }

    // Sample mesh brushes
    for brush in params.colliders {
        merge_chunk(&mut data, || {
            chunk_samples(&world_pos)
                .map(|point| brush.sample(point))
                .collect()
        });
    }

    // Apply material-specific noise
    merge_sdf_additive(&mut data, |data| {
        chunk_samples_enumerated(&world_pos)
            .map(|(i, point)| data.materials[i].sdf_noise(&point, &data.sdf[i]))
            .collect()
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
