use avian3d::prelude::*;
use bevy::prelude::*;

use super::Sampler;
use crate::{
    physics::GameLayer,
    worldgen::{
        chunk::ChunksAABB,
        voxel::{VoxelMaterial, VoxelSample},
    },
};

#[derive(Component, Clone)]
pub struct ColliderBrush {
    pub material: VoxelMaterial,
    pub width: f32,
    pub collider: Collider,
    pub transform: Transform,
}

impl Sampler for ColliderBrush {
    fn sample(&self, point: Vec3) -> VoxelSample {
        let (closest, _) = self.collider.project_point(
            Position::new(self.transform.translation),
            Rotation(self.transform.rotation),
            point,
            true,
        );

        let distance = point.distance(closest) - self.width;
        let material = self.material;

        VoxelSample { material, distance }
    }
}

#[derive(Bundle)]
pub struct ColliderBrushBundle {
    pub brush: ColliderBrush,
    pub collision_layers: CollisionLayers,
    pub chunks: ChunksAABB,
}

impl ColliderBrushBundle {
    pub fn new(width: f32, collider: Collider, transform: Transform) -> Self {
        let aabb = collider.aabb(transform.translation, Rotation(transform.rotation));
        let chunks = ChunksAABB::from_world_aabb((aabb.min, aabb.max), 0);

        Self {
            brush: ColliderBrush {
                material: VoxelMaterial::from_repr(1).unwrap(),
                width: width.max(0.01),
                collider,
                transform,
            },
            chunks,
            collision_layers: CollisionLayers::new(GameLayer::Brush, LayerMask::NONE),
        }
    }
}
