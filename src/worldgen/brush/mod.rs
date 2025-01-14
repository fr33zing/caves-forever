use bevy::{prelude::*, utils::HashSet};

use super::{consts::CHUNK_SIZE, VoxelMaterial};

pub mod collider;
pub mod curve;

#[derive(Component)]
pub struct BoundingBoxChunks {
    pub min: IVec3,
    pub max: IVec3,
    pub chunks: HashSet<IVec3>,
}

#[derive(Clone, Copy, Debug)]
pub struct VoxelSample {
    pub material: VoxelMaterial,
    pub distance: f32,
}

pub trait Sampler {
    fn sample(&self, point: Vec3) -> VoxelSample;
}

pub fn bounding_box_chunks((min, max): (Vec3, Vec3), inflate: i32) -> BoundingBoxChunks {
    let mut chunks = HashSet::new();
    let min = (min / CHUNK_SIZE as f32).floor().as_ivec3() - IVec3::splat(inflate);
    let max = (max / CHUNK_SIZE as f32).ceil().as_ivec3() + IVec3::splat(inflate);

    for x in min.x..max.x {
        for y in min.y..max.y {
            for z in min.z..max.z {
                chunks.insert(IVec3::new(x, y, z));
            }
        }
    }

    BoundingBoxChunks { min, max, chunks }
}
