use avian3d::prelude::{AnyCollider, Collider, Rotation};
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Room {
    pub source: String,
    pub weight: f32,
    pub cavities: Vec<Collider>,
    pub portals: Vec<Portal>,
    pub spawnpoints: Vec<Spawnpoint>,
}

impl Room {
    pub fn new(weight: f32, source: String) -> anyhow::Result<Room> {
        Ok(Self {
            source,
            weight,
            ..default()
        })
    }

    pub fn aabb(&self) -> (Vec3, Vec3) {
        let (mut min, mut max) = (Vec3::MAX, Vec3::MIN);
        self.cavities.iter().for_each(|cavity| {
            let aabb = cavity.aabb(Vec3::ZERO, Rotation::default());
            min.x = min.x.min(aabb.min.x);
            min.y = min.y.min(aabb.min.y);
            min.z = min.z.min(aabb.min.z);
            max.x = max.x.max(aabb.max.x);
            max.y = max.y.max(aabb.max.y);
            max.z = max.z.max(aabb.max.z);
        });

        (min, max)
    }

    pub fn inverse_world_origin_offset(&self) -> Vec3 {
        let aabb = self.aabb();
        let center = aabb.1 - aabb.0 + aabb.1 / 2.0;

        -center
    }
}

#[repr(u8)]
#[derive(
    EnumIter,
    strum::Display,
    Serialize,
    Deserialize,
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    Hash,
)]
pub enum PortalDirection {
    #[default]
    Entrance = 0,
    Exit = 1,
    Bidirectional = 2,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Portal {
    pub transform: Transform,
    pub direction: PortalDirection,
}
impl Portal {
    pub fn inward(&self) -> Vec3 {
        if self.direction == PortalDirection::Entrance {
            return *self.transform.up();
        }
        -*self.transform.up()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Spawnpoint {
    pub position: Vec3,
    pub angle: f32,
}
