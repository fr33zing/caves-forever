use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{physics::GameLayer, worldgen::VoxelMaterial};

use super::{bounding_box_chunks, BoundingBoxChunks, Sampler, VoxelSample};

#[derive(Component)]
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

        let distance = point.distance(closest);

        VoxelSample {
            material: self.material,
            distance: distance - self.width,
        }
    }
}

#[derive(Bundle)]
pub struct ColliderBrushBundle {
    pub brush: ColliderBrush,
    pub collision_layers: CollisionLayers,
    pub chunks: BoundingBoxChunks,
}

impl ColliderBrushBundle {
    pub fn new(width: f32, collider: Collider, transform: Transform) -> Self {
        let aabb = collider.aabb(transform.translation, Rotation(transform.rotation));
        let chunks = bounding_box_chunks((aabb.min, aabb.max), 0);

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
