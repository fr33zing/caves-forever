use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
};
use fast_surface_nets::{ndshape::ConstShape, surface_nets, SurfaceNetsBuffer};
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::{
    materials::{ATTRIBUTE_VOXEL_RATIO, ATTRIBUTE_VOXEL_TYPE},
    worldgen::voxel::{VoxelMaterial, VoxelSample},
};

use super::{
    ChunkData, ChunkShape, CHUNK_INTERNAL_GEOMETRY, CHUNK_SAMPLE_RESOLUTION, CHUNK_SAMPLE_SIZE,
    CHUNK_SIZE_F,
};

pub fn copy_sdf_plane(
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
pub fn copy_borders(a: &mut ChunkData, b: &ChunkData) -> bool {
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

pub fn delinearize_to_world_pos(chunk_world_pos: Vec3, sample: u32) -> Vec3 {
    let [x, y, z] = ChunkShape::delinearize(sample);
    let point = Vec3::new(x as f32, y as f32, z as f32);
    point / CHUNK_SAMPLE_RESOLUTION + chunk_world_pos
}

pub fn chunk_samples(
    chunk_world_pos: &Vec3,
) -> rayon::iter::Map<rayon::range::Iter<u32>, impl Fn(u32) -> Vec3> {
    let chunk_world_pos = chunk_world_pos.clone();
    (0u32..ChunkShape::SIZE)
        .into_par_iter()
        .map(move |i| delinearize_to_world_pos(chunk_world_pos, i))
}

// This function will probably come in handy at some point, so I'll keep it for now.
#[allow(dead_code)]
pub fn merge_sdf<F>(sdf: &mut [f32; ChunkShape::USIZE], sampler: F) -> bool
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
pub fn merge_sdf_with_hardness<F>(data: &mut ChunkData, force: f32, sampler: F) -> bool
where
    F: Fn() -> Vec<f32>,
{
    let mut changed = false;
    let new_sdf = sampler();

    for (i, distance) in new_sdf.into_iter().enumerate() {
        if distance < data.sdf[i] {
            // TODO fix hardness
            // let hardness = data.materials[i].hardness().multiplier();
            // let difference = data.sdf[i] - distance;
            // data.sdf[i] -= difference * force / hardness;

            data.sdf[i] = distance;

            changed = true;
        }
    }

    changed
}

pub fn postprocess_sample(sample: &mut VoxelSample) {
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

pub fn merge_chunk<F>(data: &mut ChunkData, sampler: F)
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

pub fn mesh_chunk(data: &ChunkData) -> Option<(Mesh, Collider)> {
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
