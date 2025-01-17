use std::sync::{Arc, Mutex};

use avian3d::prelude::*;
use bevy::{
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};

use super::{utility::*, TerrainState, TerrainStateResource};

#[derive(Default, Clone)]
pub struct ChunkRemeshParams {
    pub state: Arc<Mutex<TerrainState>>,
    pub chunk_pos: IVec3,
}

impl ChunkRemeshParams {
    pub fn new(state: Arc<Mutex<TerrainState>>) -> Self {
        Self { state, ..default() }
    }

    pub fn clone_for(&self, chunk_pos: &IVec3) -> Self {
        let mut clone = self.clone();
        clone.chunk_pos = chunk_pos.clone();
        clone
    }
}

pub struct ChunkRemeshResult(pub Mesh, pub Collider);

#[derive(Component)]
pub struct ChunkRemeshTask(Task<Option<ChunkRemeshResult>>, Entity);

pub fn begin_remesh_chunks(mut commands: Commands, state: Res<TerrainStateResource>) {
    let task_pool = AsyncComputeTaskPool::get();
    let request = ChunkRemeshParams::new(state.clone());

    let state = state.clone();
    let mut state = state.lock().unwrap();

    if state.chunks_to_remesh.len() == 0 {
        return;
    }

    state.chunks_to_remesh.iter().for_each(|(chunk, entity)| {
        let request = request.clone_for(chunk);
        let task = task_pool.spawn(async move { remesh_chunk(request) });
        commands.spawn(ChunkRemeshTask(task, *entity));
    });

    state.chunks_to_remesh.clear();
}

pub fn receive_remesh_chunks(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tasks: Query<(Entity, &mut ChunkRemeshTask)>,
) {
    for (task_entity, mut task) in tasks.iter_mut() {
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
    let state = params.state.clone();
    let state = state.lock().unwrap();

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
