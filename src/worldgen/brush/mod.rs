use bevy::math::Vec3;

use super::voxel::VoxelSample;

pub mod collider;
pub mod curve;

pub trait Sampler {
    fn sample(&self, point: Vec3) -> VoxelSample;
}
