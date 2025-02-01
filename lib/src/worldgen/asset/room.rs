use avian3d::prelude::Collider;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Room {
    pub weight: f32,
    pub cavities: Vec<Collider>,
    pub portals: Vec<Portal>,
    pub spawnpoints: Vec<Spawnpoint>,
}

impl Room {
    pub fn new(weight: f32) -> Room {
        Self {
            weight,
            ..default()
        }
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

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Spawnpoint {
    pub position: Vec3,
    pub angle: f32,
}
