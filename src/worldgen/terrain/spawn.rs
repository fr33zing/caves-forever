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
    worldgen::brush::{collider::ColliderBrush, curve::CurveBrush, Sampler},
};

use super::{
    utility::*, Chunk, ChunkData, DestroyTerrain, TerrainState, TerrainStateResource,
    CHUNK_SAMPLE_RESOLUTION, CHUNK_SIZE_F,
};

#[derive(Default, Clone)]
pub struct SpawnChunkRequest {
    pub chunk_pos: IVec3,
    pub copy_borders: bool,
    pub destruction: Option<Vec<DestroyTerrain>>,
}

#[derive(Default, Clone)]
pub struct ChunkGenParams {
    pub state: Arc<Mutex<TerrainState>>,
    pub request: SpawnChunkRequest,
    pub curves: Vec<CurveBrush>,
    pub colliders: Vec<ColliderBrush>,
}

impl ChunkGenParams {
    pub fn new(state: Arc<Mutex<TerrainState>>) -> Self {
        Self { state, ..default() }
    }

    pub fn clone_for(&self, spawn_chunk: &SpawnChunkRequest) -> Self {
        let mut clone = self.clone();
        clone.request = spawn_chunk.clone();

        clone
    }
}

pub struct ChunkGenResult {
    pub data: ChunkData,
    pub mesh: Mesh,
    pub collider: Collider,
}

#[derive(Component)]
pub struct ChunkGenTask(Task<Option<ChunkGenResult>>);

pub fn begin_spawn_chunks(
    mut commands: Commands,
    state: Res<TerrainStateResource>,
    curve_brush_query: Query<&CurveBrush>,
    collider_brush_query: Query<&ColliderBrush>,
) {
    let mut params = ChunkGenParams::new(state.clone());

    let state = state.clone();
    let mut state = state.lock().unwrap();

    if state.chunks_to_spawn.len() == 0 {
        return;
    }

    curve_brush_query.iter().for_each(|brush| {
        params.curves.push(brush.clone());
    });
    collider_brush_query.iter().for_each(|brush| {
        params.colliders.push(brush.clone());
    });

    let task_pool = AsyncComputeTaskPool::get();

    state.chunks_to_spawn.iter().for_each(|chunk| {
        let params = params.clone_for(&chunk);
        let task = task_pool.spawn(async move { spawn_chunks(params) });
        commands.spawn(ChunkGenTask(task));
    });

    state.chunks_to_spawn.clear();
}

pub fn receive_spawn_chunks(
    mut commands: Commands,
    state: Res<TerrainStateResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, CaveMaterialExtension>>>,
    mut tasks: Query<(Entity, &mut ChunkGenTask)>,
) {
    for (entity, mut task) in tasks.iter_mut() {
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

            let state = state.clone();
            let mut state = state.lock().unwrap();
            state
                .chunk_data
                .insert(generated.data.chunk_pos, (generated.data, entity));
        }

        let mut commands = commands.entity(entity);
        commands.clear();
    }
}

pub fn spawn_chunks(params: ChunkGenParams) -> Option<ChunkGenResult> {
    let mut data = ChunkData::new(params.request.chunk_pos);
    let world_pos = data.world_pos();

    // Don't spawn chunks where they already exist
    if cfg!(debug_assertions) {
        let state = params.state.clone();
        let mut state = state.lock().unwrap();

        for (_, (other, _)) in state.chunk_data.iter_mut() {
            if other.chunk_pos == params.request.chunk_pos {
                panic!("tried to spawn chunk where one already exists");
            }
        }
    }

    // Sample curve brushes
    for brush in params.curves.iter() {
        merge_chunk(&mut data, || {
            chunk_samples(&world_pos)
                .map(|point| brush.sample(point))
                .collect()
        });
    }

    // Sample mesh brushes
    for brush in params.colliders.iter() {
        merge_chunk(&mut data, || {
            chunk_samples(&world_pos)
                .map(|point| brush.sample(point))
                .collect()
        });
    }

    if let Some(destruction) = params.request.destruction {
        for destroy in destruction.iter() {
            merge_sdf_with_hardness(&mut data, destroy.force, || {
                chunk_samples(&world_pos)
                    .map(|point| point.distance(destroy.position) - destroy.radius)
                    .collect()
            });
        }
    }

    if params.request.copy_borders {
        let state = params.state.clone();
        let mut state = state.lock().unwrap();

        let mut chunks_to_remesh = Vec::<(IVec3, Entity)>::new();
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
                chunks_to_remesh.push((neighbor.chunk_pos, *entity));
            }
        }

        state.chunks_to_remesh.extend(chunks_to_remesh);
    }

    let Some((mesh, collider)) = mesh_chunk(&data) else {
        return None;
    };

    Some(ChunkGenResult {
        data,
        mesh,
        collider,
    })
}
