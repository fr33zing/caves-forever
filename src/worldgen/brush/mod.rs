use bevy::{prelude::*, utils::HashSet};

use super::{consts::CHUNK_SIZE, VoxelMaterial};

pub mod collider;
pub mod curve;

#[derive(Component)]
pub struct ChunksAABB {
    pub chunks: HashSet<IVec3>,
    pub min: IVec3,
    pub max: IVec3,
}

impl ChunksAABB {
    pub fn chunks(min: IVec3, max: IVec3) -> HashSet<IVec3> {
        let mut chunks = HashSet::new();

        for x in min.x..max.x {
            for y in min.y..max.y {
                for z in min.z..max.z {
                    chunks.insert(IVec3::new(x, y, z));
                }
            }
        }

        chunks
    }

    pub fn from_world_aabb((min, max): (Vec3, Vec3), inflate: i32) -> ChunksAABB {
        let inflate = IVec3::splat(inflate);
        let min = (min / CHUNK_SIZE as f32).floor().as_ivec3() - inflate;
        let max = (max / CHUNK_SIZE as f32).ceil().as_ivec3() + inflate;
        let chunks = Self::chunks(min, max);

        ChunksAABB { min, max, chunks }
    }

    pub fn inflated(&self, inflate: i32) -> ChunksAABB {
        let inflate = IVec3::splat(inflate);
        let min = self.min - inflate;
        let max = self.max + inflate;
        let chunks = Self::chunks(min, max);

        ChunksAABB { min, max, chunks }
    }

    pub fn inflate(&mut self, inflate: i32) {
        let inflate = IVec3::splat(inflate);
        self.min -= inflate;
        self.max += inflate;
        self.chunks = Self::chunks(self.min, self.max);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct VoxelSample {
    pub material: VoxelMaterial,
    pub distance: f32,
}

pub trait Sampler {
    fn sample(&self, point: Vec3) -> VoxelSample;
}
