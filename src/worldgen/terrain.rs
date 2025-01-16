use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    ecs::system::SystemState,
    math::Vec3A,
    pbr::{ExtendedMaterial, OpaqueRendererMethod},
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
        primitives::Aabb,
    },
    utils::HashSet,
};
use fast_surface_nets::{
    ndshape::{ConstShape, ConstShape3u32},
    surface_nets, SurfaceNetsBuffer,
};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use super::{
    brush::{collider::ColliderBrush, curve::*, Sampler},
    chunk::ChunksAABB,
    consts::*,
    layout,
    voxel::{VoxelHardness, VoxelMaterial, VoxelSample},
};
use crate::{
    materials::{CaveMaterialExtension, ATTRIBUTE_VOXEL_RATIO, ATTRIBUTE_VOXEL_TYPE},
    physics::GameLayer,
};

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

#[derive(Component, Clone)]
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

//
// Plugin
//

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<DestroyTerrainEvent>()
            .add_systems(Startup, (setup, layout::setup_debug_layout.before(setup)))
            .add_systems(Update, (draw_debug, destroy_terrain));
    }
}

//
// Events
//

#[derive(Event)]
pub struct DestroyTerrainEvent {
    pub position: Vec3,
    pub radius: f32,
    pub force: f32,
}

impl DestroyTerrainEvent {
    fn world_extents(&self) -> (Vec3, Vec3) {
        let inflate = 1.0; // World units, not chunks
        let radius = Vec3::splat(self.radius + inflate);
        let min = self.position - radius;
        let max = self.position + radius;

        (min, max)
    }
}

//
// Systems
//

fn setup(mut commands: Commands, aabb_query: Query<&ChunksAABB>) {
    let mut chunks = HashSet::<IVec3>::new();

    for aabb in aabb_query.iter() {
        chunks.extend(&aabb.chunks);
    }

    for chunk_pos in chunks {
        let data = ChunkData::new(chunk_pos);
        commands.queue(SpawnChunkCommand(data, false));
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

    gizmos.axes(
        Transform::from_translation(Vec3::splat(0.125)),
        CHUNK_SIZE_F,
    );
}

fn sample_all_chunks(world_pos: Vec3, chunk_query: Query<&ChunkData>) -> Option<VoxelSample> {
    let mut sample: Option<VoxelSample> = None;

    let voxel_pos = (world_pos / CHUNK_SAMPLE_SIZE_F).floor() * CHUNK_SAMPLE_SIZE_F;
    let max = CHUNK_SAMPLE_SIZE_F;

    for data in chunk_query.iter() {
        let voxel_pos = voxel_pos - (data.world_pos() / max).floor() * max;

        if voxel_pos.x < 0.0
            || voxel_pos.x > max
            || voxel_pos.y < 0.0
            || voxel_pos.y > max
            || voxel_pos.z < 0.0
            || voxel_pos.z > max
        {
            // Point is outside chunk
            continue;
        }

        let voxel_pos = voxel_pos.as_uvec3();
        let i = ChunkShape::linearize([voxel_pos.x, voxel_pos.y, voxel_pos.z]) as usize;
        sample = Some(VoxelSample {
            distance: data.sdf[i],
            material: data.materials[i],
        });

        break;
    }

    sample
}

fn destroy_terrain(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut event: EventReader<DestroyTerrainEvent>,
    mut chunk_query: Query<(Entity, &mut ChunkData)>,
) {
    let events: Vec<(&DestroyTerrainEvent, ChunksAABB, ChunksAABB)> = event
        .read()
        .map(|e| {
            let aabb = ChunksAABB::from_world_aabb(e.world_extents(), 0);
            let aabb_inflated = aabb.inflated(1);

            (e, aabb, aabb_inflated)
        })
        .collect();

    if events.len() == 0 {
        return;
    }

    let mut chunks_to_generate: HashSet<IVec3> =
        events.iter().flat_map(|e| e.1.chunks.clone()).collect();

    chunk_query.iter_mut().for_each(|(entity, mut data)| {
        let mut changed = false;

        chunks_to_generate.remove(&data.chunk_pos);

        for (e, _, aabb_inflated) in events.iter() {
            // TODO ensure this optimization can't result in non-manifold geometry
            if !aabb_inflated.chunks.contains(&data.chunk_pos) {
                continue;
            }

            let world_pos = data.world_pos();

            changed = changed
                || merge_sdf_with_hardness(&mut data, e.force, || {
                    chunk_samples(&world_pos)
                        .map(|point| e.position.distance(point) - e.radius)
                        .collect()
                });
        }

        if changed {
            if let Some((mesh, collider)) = mesh_chunk(&data) {
                let mut commands = commands.entity(entity);
                commands.remove::<Collider>();
                commands.insert(collider);
                commands.remove::<Mesh3d>();
                commands.insert(Mesh3d(meshes.add(mesh)));
            } else {
                commands.entity(entity).clear();
            }
        }
    });

    for chunk_pos in chunks_to_generate {
        for (e, _, aabb_inflated) in events.iter() {
            // TODO ensure this optimization can't result in non-manifold geometry
            if !aabb_inflated.chunks.contains(&chunk_pos) {
                continue;
            }

            let mut data = ChunkData::new(chunk_pos);
            let world_pos = data.world_pos();

            merge_sdf_with_hardness(&mut data, e.force, || {
                chunk_samples(&world_pos)
                    .map(|point| e.position.distance(point) - e.radius)
                    .collect()
            });

            commands.queue(SpawnChunkCommand(data, true));
        }
    }
}

//
// Commands
//

/// Spawns a new chunk, optionally copying border data from adjacent chunks.
struct SpawnChunkCommand(ChunkData, bool);

struct RemeshChunkCommand(Entity);

impl Command for SpawnChunkCommand {
    fn apply(mut self, world: &mut World) {
        let mut system_state: SystemState<(
            Commands,
            ResMut<Assets<Mesh>>,
            ResMut<Assets<ExtendedMaterial<StandardMaterial, CaveMaterialExtension>>>,
            Query<&CurveBrush>,
            Query<&ColliderBrush>,
            Query<(Entity, &mut ChunkData)>,
        )> = SystemState::new(world);
        let (
            mut commands,
            mut meshes,
            mut materials,
            curve_brush_query,
            collider_brush_query,
            mut chunk_query,
        ) = system_state.get_mut(world);

        let world_pos = self.0.world_pos();

        // Sample curve brushes
        for brush in curve_brush_query.iter() {
            merge_chunk(&mut self.0, || {
                chunk_samples(&world_pos)
                    .map(|point| brush.sample(point))
                    .collect()
            });
        }

        // Sample mesh brushes
        for brush in collider_brush_query.iter() {
            merge_chunk(&mut self.0, || {
                chunk_samples(&world_pos)
                    .map(|point| brush.sample(point))
                    .collect()
            });
        }

        // Copy borders from adjacent chunks
        let mut remesh_commands = Vec::<RemeshChunkCommand>::new();
        if self.1 {
            chunk_query.iter_mut().for_each(|(entity, data)| {
                copy_borders(&mut self.0, &data);
                remesh_commands.push(RemeshChunkCommand(entity));
            });
        }

        if let Some((mesh, collider)) = mesh_chunk(&self.0) {
            let scale = Vec3::splat(1.0 / CHUNK_SAMPLE_RESOLUTION);
            let half_extents = Vec3A::splat(CHUNK_SIZE_F / 2.0);

            commands.spawn((
                self.0,
                collider,
                Chunk,
                Aabb {
                    center: half_extents,
                    half_extents,
                },
                Transform::from_translation(world_pos).with_scale(scale),
                RigidBody::Static,
                CollisionLayers::new(GameLayer::World, LayerMask::ALL),
                DebugRender::default().without_collider().without_axes(),
                Mesh3d(meshes.add(mesh)),
                MeshMaterial3d(materials.add(ExtendedMaterial {
                    base: StandardMaterial {
                        base_color: Color::srgb(0.5, 0.5, 0.5),
                        opaque_render_method: OpaqueRendererMethod::Auto,
                        ..Default::default()
                    },
                    extension: CaveMaterialExtension::new(7.0, 5.0),
                })),
            ));
        }

        remesh_commands.into_iter().for_each(|c| commands.queue(c));
        system_state.apply(world);
    }
}

impl Command for RemeshChunkCommand {
    // This can probably be optimized by tracking if copying the borders is actually needed
    fn apply(self, world: &mut World) {
        let mut system_state: SystemState<(
            Commands,
            ResMut<Assets<Mesh>>,
            Query<(Entity, &mut ChunkData)>,
        )> = SystemState::new(world);
        let (mut commands, mut meshes, mut chunk_query) = system_state.get_mut(world);

        let mut combinations = chunk_query.iter_combinations_mut();
        let mut changed = false;
        while let Some([mut a, mut b]) = combinations.fetch_next() {
            if a.0 == self.0 {
                changed = changed || copy_borders(&mut a.1, &b.1);
            } else if b.0 == self.0 {
                changed = changed || copy_borders(&mut b.1, &a.1);
            }
        }

        if !changed {
            return;
        }

        let data = chunk_query.get(self.0).unwrap().1;
        if let Some((mesh, collider)) = mesh_chunk(&data) {
            let mut commands = commands.entity(self.0);
            commands.remove::<Collider>();
            commands.insert(collider);
            commands.remove::<Mesh3d>();
            commands.insert(Mesh3d(meshes.add(mesh)));
        } else {
            commands.entity(self.0).clear();
        }

        system_state.apply(world);
    }
}

//
// Utility
//

fn copy_sdf_plane(
    a: &mut ChunkData,
    b: &ChunkData,
    axis0: usize,
    axis1: usize,
    offset0: u32,
    offset1: u32,
) -> bool {
    let mut changed = false;
    let max = CHUNK_SAMPLE_SIZE + 1;

    for axis_point_0 in 0..=max {
        for axis_point_1 in 0..=max {
            let mut point0 = [offset0, offset0, offset0];
            point0[axis0] = axis_point_0;
            point0[axis1] = axis_point_1;
            let mut point1 = [offset1, offset1, offset1];
            point1[axis0] = axis_point_0;
            point1[axis1] = axis_point_1;

            let i = ChunkShape::linearize(point0) as usize;
            let j = ChunkShape::linearize(point1) as usize;

            if !changed && (a.sdf[i] != b.sdf[j] || a.materials[i] != b.materials[j]) {
                changed = true;
            }

            a.sdf[i] = b.sdf[j];
            a.materials[i] = b.materials[j];
        }
    }

    changed
}

/// Returns true if chunks are adjacent
fn copy_borders(a: &mut ChunkData, b: &ChunkData) -> bool {
    let dir = a.chunk_pos - b.chunk_pos;
    let min = 0;
    let max = CHUNK_SAMPLE_SIZE + 1;

    match dir {
        IVec3 { x: -1, y: 0, z: 0 } => copy_sdf_plane(a, &b, 1, 2, max, min + 1),
        IVec3 { x: 1, y: 0, z: 0 } => copy_sdf_plane(a, &b, 1, 2, min, max - 1),
        IVec3 { x: 0, y: -1, z: 0 } => copy_sdf_plane(a, &b, 0, 2, max, min + 1),
        IVec3 { x: 0, y: 1, z: 0 } => copy_sdf_plane(a, &b, 0, 2, min, max - 1),
        IVec3 { x: 0, y: 0, z: -1 } => copy_sdf_plane(a, &b, 0, 1, max, min + 1),
        IVec3 { x: 0, y: 0, z: 1 } => copy_sdf_plane(a, &b, 0, 1, min, max - 1),
        _ => false,
    }
}

fn delinearize_to_world_pos(chunk_world_pos: Vec3, sample: u32) -> Vec3 {
    let [x, y, z] = ChunkShape::delinearize(sample);
    let point = Vec3::new(x as f32, y as f32, z as f32);
    point / CHUNK_SAMPLE_RESOLUTION + chunk_world_pos
}

fn chunk_samples(
    chunk_world_pos: &Vec3,
) -> rayon::iter::Map<rayon::range::Iter<u32>, impl Fn(u32) -> Vec3> {
    let chunk_world_pos = chunk_world_pos.clone();
    (0u32..ChunkShape::SIZE)
        .into_par_iter()
        .map(move |i| delinearize_to_world_pos(chunk_world_pos, i))
}

// This function will probably come in handy at some point, so I'll keep it for now.
#[allow(dead_code)]
fn merge_sdf<F>(sdf: &mut [f32; ChunkShape::USIZE], sampler: F) -> bool
where
    F: Fn() -> Vec<f32>,
{
    let mut changed = false;
    let new_sdf = sampler();

    for (i, distance) in new_sdf.into_iter().enumerate() {
        if distance < sdf[i] {
            sdf[i] = distance;
            changed = true;
        }
    }

    changed
}

// TODO ensure this can't result in non-manifold geometry
// TODO consider hardness of the hit material to prevent destroying soft materials behind hard materials
fn merge_sdf_with_hardness<F>(data: &mut ChunkData, force: f32, sampler: F) -> bool
where
    F: Fn() -> Vec<f32>,
{
    let mut changed = false;
    let new_sdf = sampler();

    for (i, distance) in new_sdf.into_iter().enumerate() {
        if distance < data.sdf[i] {
            let multiplier = data.materials[i].hardness().multiplier() / force;

            let difference = data.sdf[i] - distance;
            data.sdf[i] = distance + difference * (1.0 - (1.0 / multiplier));

            changed = true;
        }
    }

    changed
}

fn postprocess_sample(sample: &mut VoxelSample) {
    if sample.distance > 50.0 {
        if sample.distance > 100.0 {
            if sample.distance > 104.0 {
                sample.material = VoxelMaterial::Boundary;
            } else {
                sample.material = VoxelMaterial::FakeBoundary;
            }
        } else {
            sample.material = VoxelMaterial::ShinyGreenRock;
        }
    }
}

fn merge_chunk<F>(data: &mut ChunkData, sampler: F)
where
    F: Fn() -> Vec<VoxelSample>,
{
    let mut new_sdf = sampler();
    for (i, sample) in new_sdf.iter_mut().enumerate() {
        if sample.distance < data.sdf[i] {
            postprocess_sample(sample);
            data.sdf[i] = sample.distance;
            data.materials[i] = sample.material;
        } else if data.materials[i] == VoxelMaterial::Unset {
            postprocess_sample(sample);
            data.materials[i] = sample.material;
        }
    }
}

fn mesh_chunk(data: &ChunkData) -> Option<(Mesh, Collider)> {
    let mut sdf = data.sdf.clone();

    if CHUNK_INTERNAL_GEOMETRY {
        for i in 0..ChunkShape::USIZE {
            sdf[i] = -sdf[i];
        }
    }

    let mut buffer = SurfaceNetsBuffer::default();
    surface_nets(
        &sdf,
        &ChunkShape {},
        [0; 3],
        [CHUNK_SAMPLE_SIZE + 1; 3],
        &mut buffer,
    );

    if buffer.positions.len() < 3 || buffer.indices.len() < 3 {
        return None;
    }

    let mut physics_mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all());
    physics_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, buffer.positions);
    physics_mesh.insert_indices(Indices::U32(buffer.indices));

    let collider = Collider::trimesh_from_mesh_with_config(
        &physics_mesh,
        TrimeshFlags::MERGE_DUPLICATE_VERTICES,
    )
    .unwrap();

    // Unconnected triangles are required to blend voxel types
    let mut render_mesh = physics_mesh.clone();
    render_mesh.duplicate_vertices();
    render_mesh.compute_flat_normals();

    let positions = render_mesh
        .attribute(Mesh::ATTRIBUTE_POSITION)
        .unwrap()
        .as_float3()
        .unwrap();
    let voxel_types: Vec<u8> = positions
        .iter()
        .map(|pos| {
            let index = ChunkShape::linearize([
                pos[0].floor() as u32,
                pos[1].floor() as u32,
                pos[2].floor() as u32,
            ]);
            data.materials[index as usize] as u8
        })
        .collect();
    let voxel_types: Vec<[u8; 4]> = (0..(positions.len() / 3))
        .flat_map(|i| {
            let a = voxel_types[i * 3];
            let b = voxel_types[i * 3 + 1];
            let c = voxel_types[i * 3 + 2];
            vec![[a, b, c, 0], [a, b, c, 0], [a, b, c, 0]]
        })
        .collect();
    let voxel_ratios: Vec<[f32; 3]> = (0..positions.len())
        .map(|i| match i % 3 {
            0 => [1.0, 0.0, 0.0],
            1 => [0.0, 1.0, 0.0],
            _ => [0.0, 0.0, 1.0],
        })
        .collect();

    render_mesh.insert_attribute(ATTRIBUTE_VOXEL_RATIO, voxel_ratios);
    render_mesh.insert_attribute(
        ATTRIBUTE_VOXEL_TYPE,
        VertexAttributeValues::Uint8x4(voxel_types),
    );

    Some((render_mesh, collider))
}
