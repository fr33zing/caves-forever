use avian3d::prelude::{AnyCollider, Collider, Rotation};
use bevy::prelude::*;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RoomFlags(u8);

bitflags! {
    impl RoomFlags: u8 {
        const Spawnable = 1;
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Room {
    pub flags: RoomFlags,
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
        let center = aabb.0 + ((aabb.1 - aabb.0) / 2.0);

        -center
    }

    pub fn radius(&self) -> f32 {
        let (min, max) = self.aabb();
        max.distance(min) / 2.0
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
impl PortalDirection {
    pub fn is_entrance(&self) -> bool {
        match self {
            PortalDirection::Entrance => true,
            PortalDirection::Exit => false,
            PortalDirection::Bidirectional => true,
        }
    }

    pub fn is_exit(&self) -> bool {
        match self {
            PortalDirection::Entrance => false,
            PortalDirection::Exit => true,
            PortalDirection::Bidirectional => true,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Portal {
    pub transform: Transform,
    pub direction: PortalDirection,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Spawnpoint {
    pub position: Vec3,
    pub angle: f32,
}
