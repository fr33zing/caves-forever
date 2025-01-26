use avian3d::prelude::*;
use bevy::prelude::*;

use super::{
    chunk::ChunksAABB,
    voxel::{VoxelMaterial, VoxelSample},
};

pub mod collider;
pub mod curve;
pub mod sweep;

use curve::CurveBrush;

pub trait Sampler {
    fn sample(&self, point: Vec3) -> VoxelSample;
}

#[derive(Component, Clone)]
pub enum TerrainBrush {
    Curve(CurveBrush, VoxelMaterial, ChunksAABB),
    Collider(Collider, VoxelMaterial, ChunksAABB),
}

impl TerrainBrush {
    pub fn chunks(&self) -> &ChunksAABB {
        match self {
            TerrainBrush::Curve(_, _, chunks_aabb) => chunks_aabb,
            TerrainBrush::Collider(_, _, chunks_aabb) => chunks_aabb,
        }
    }

    pub fn sample(&self, point: Vec3) -> VoxelSample {
        match self {
            TerrainBrush::Curve(_, _, _) => todo!(),
            TerrainBrush::Collider(_, _, _) => self.sample_collider(point),
        }
    }

    fn sample_collider(&self, point: Vec3) -> VoxelSample {
        let TerrainBrush::Collider(collider, material, _) = self else {
            panic!();
        };

        let (closest, _) =
            collider.project_point(Position::default(), Rotation::default(), point, false);
        let (closest_solid, _) =
            collider.project_point(Position::default(), Rotation::default(), point, true);

        let mut distance = point.distance(closest);

        // is_inside from project_point is unreliable
        let is_inside = closest_solid.distance(point) <= 0.01;

        if is_inside {
            distance *= -1.0;
            distance = distance.min(-0.001);
        }

        VoxelSample {
            material: *material,
            distance,
        }
    }
}
