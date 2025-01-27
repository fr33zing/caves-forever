use std::sync::{Arc, Mutex};

use bevy::{
    prelude::*,
    utils::{HashMap, HashSet},
};
use boundary::enforce_loading_chunk_boundaries;
use fast_surface_nets::ndshape::{ConstShape, ConstShape3u32};

use super::{
    brush::TerrainBrushPlugin, chunk::ChunksAABB, consts::*, layout, voxel::VoxelMaterial,
};

mod boundary;
mod change_detection;
mod destroy;
mod remesh;
mod spawn;
mod utility;

use change_detection::TerrainChangeDetectionPlugin;
use destroy::*;
use remesh::*;
use spawn::*;
use utility::*;

pub use destroy::DestroyTerrainEvent;

//
// Types & consts
//

type ChunkShape =
    ConstShape3u32<{ CHUNK_SAMPLE_SIZE + 2 }, { CHUNK_SAMPLE_SIZE + 2 }, { CHUNK_SAMPLE_SIZE + 2 }>;

const CHUNK_BORDER_INSET: f32 = 0.0;

//
// Structs
//

#[derive(Component)]
pub struct Chunk;

pub struct ChunkData {
    chunk_pos: IVec3,
    materials: [VoxelMaterial; ChunkShape::USIZE],
    sdf: [f32; ChunkShape::USIZE],
}

impl ChunkData {
    fn new(chunk_pos: IVec3) -> Self {
        Self {
            chunk_pos,
            materials: [VoxelMaterial::Unset; ChunkShape::USIZE],
            sdf: [f32::MAX; ChunkShape::USIZE],
        }
    }

    pub fn world_pos(&self) -> Vec3 {
        self.chunk_pos.as_vec3() * CHUNK_SIZE_F
    }
}

#[derive(Resource, Default, Deref)]
struct TerrainStateMutex(pub Arc<Mutex<TerrainState>>);

#[derive(Default)]
struct TerrainState {
    pub chunk_data: HashMap<IVec3, (ChunkData, Entity)>,

    pub spawn_requests: Vec<ChunkSpawnRequest>,
    pub remesh_requests: Vec<ChunkRemeshRequest>,
}

impl TerrainState {
    /// Returns up to 4 neighboring chunks
    pub fn neighbors(&self, chunk_pos: &IVec3) -> Vec<IVec3> {
        let directions: Vec<IVec3> = vec![
            IVec3 { x: -1, y: 0, z: 0 },
            IVec3 { x: 1, y: 0, z: 0 },
            IVec3 { x: 0, y: -1, z: 0 },
            IVec3 { x: 0, y: 1, z: 0 },
            IVec3 { x: 0, y: 0, z: -1 },
            IVec3 { x: 0, y: 0, z: 1 },
        ];

        directions
            .iter()
            .filter_map(|d| {
                let key = d + chunk_pos;
                if !self.chunk_data.contains_key(&key) {
                    return None;
                }
                Some(key)
            })
            .collect()
    }
}

//
// Plugin
//

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TerrainStateMutex>()
            .add_event::<DestroyTerrainEvent>()
            .add_plugins((TerrainChangeDetectionPlugin, TerrainBrushPlugin))
            .add_systems(Startup, (setup, layout::setup_debug_layout.before(setup)))
            .add_systems(Update, draw_debug)
            .add_systems(Update, enforce_loading_chunk_boundaries)
            .add_systems(
                Update,
                (
                    begin_remesh_chunks,
                    receive_remesh_chunks,
                    begin_spawn_chunks,
                    receive_spawn_chunks,
                    begin_destroy_terrain,
                )
                    .chain(),
            );
    }
}

fn setup(state: Res<TerrainStateMutex>, aabb_query: Query<&ChunksAABB>) {
    let mut chunks = HashSet::<IVec3>::new();

    for aabb in aabb_query.iter() {
        chunks.extend(&aabb.chunks);
    }

    let state = (*state).clone();
    let mut state = state.lock().unwrap();

    for chunk_pos in chunks {
        state.spawn_requests.push(ChunkSpawnRequest {
            chunk_pos,
            copy_borders: false,
            ..default()
        });
    }
}

fn draw_debug(mut gizmos: Gizmos, chunk_query: Query<&Transform, With<Chunk>>) {
    if CHUNK_RENDER_BORDERS {
        for transform in chunk_query.iter() {
            gizmos.cuboid(
                Transform::from_translation(
                    (*transform).translation + Vec3::splat(CHUNK_SIZE_F / 2.0 + CHUNK_BORDER_INSET),
                )
                .with_scale(Vec3::splat(CHUNK_SIZE_F - CHUNK_BORDER_INSET * 2.0)),
                Color::srgba(0.0, 0.0, 0.0, 0.25),
            );
        }
    }

    if WORLD_RENDER_ORIGIN {
        gizmos.axes(
            Transform::from_translation(Vec3::splat(0.125)),
            CHUNK_SIZE_F,
        );
    }
}
