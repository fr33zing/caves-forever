use std::sync::{Arc, Mutex};

use avian3d::prelude::*;
use bevy::{
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};

use super::{utility::*, TerrainState, TerrainStateMutex};

pub struct ChunkRemeshRequest {
    pub chunk_pos: IVec3,
    pub chunk_entity: Entity,
}

#[derive(Default, Clone)]
struct ChunkRemeshParams {
    state: Arc<Mutex<TerrainState>>,
    chunk_pos: IVec3,
}

impl ChunkRemeshParams {
    fn new(state: Arc<Mutex<TerrainState>>) -> Self {
        Self { state, ..default() }
    }

    fn with_request(&self, request: &ChunkRemeshRequest) -> Self {
        let mut clone = self.clone();
        clone.chunk_pos = request.chunk_pos.clone();
        clone
    }
}

struct ChunkRemeshResult(Mesh, Collider);

#[derive(Component)]
pub struct ChunkRemeshTask(Task<Option<ChunkRemeshResult>>, Entity);

pub fn begin_remesh_chunks(mut commands: Commands, state: Res<TerrainStateMutex>) {
    let task_pool = AsyncComputeTaskPool::get();
    let params = ChunkRemeshParams::new(state.clone());
    let mut state = state.lock().unwrap();

    if state.remesh_requests.len() == 0 {
        return;
    }

    state.remesh_requests.iter().for_each(|request| {
        let params = params.with_request(&request);
        let task = task_pool.spawn(async move { remesh_chunk(params) });
        commands.spawn(ChunkRemeshTask(task, request.chunk_entity));
    });

    state.remesh_requests.clear();
}

pub fn receive_remesh_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut remesh_tasks: Query<(Entity, &mut ChunkRemeshTask)>,
) {
    for (task_entity, mut task) in remesh_tasks.iter_mut() {
        let status = block_on(future::poll_once(&mut task.0));

        let Some(result) = status else {
            continue;
        };

        if let Some(ChunkRemeshResult(mesh, collider)) = result {
            let mut commands = commands.entity(task.1);
            commands.remove::<Collider>();
            commands.insert(collider);
            commands.remove::<Mesh3d>();
            commands.insert(Mesh3d(meshes.add(mesh)));
        } else {
            commands.entity(task.1).clear();
        }

        let mut commands = commands.entity(task_entity);
        commands.clear();
    }
}

fn remesh_chunk(params: ChunkRemeshParams) -> Option<ChunkRemeshResult> {
    let state = params.state.lock().unwrap();

    let Some((data, _)) = state.chunk_data.get(&params.chunk_pos) else {
        if cfg!(debug_assertions) {
            panic!("tried to remesh nonexistent chunk");
        }
        return None;
    };

    let Some((mesh, collider)) = mesh_chunk(&data) else {
        return None;
    };

    Some(ChunkRemeshResult(mesh, collider))
}
