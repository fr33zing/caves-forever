use avian3d::prelude::Collider;
use bevy::prelude::Transform;
use serde::{Deserialize, Serialize};
use strum::EnumIter;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Room {
    pub weight: f32,
    pub cavities: Vec<Collider>,
    pub portals: Vec<Portal>,
}

#[repr(u8)]
#[derive(
    EnumIter, strum::Display, Serialize, Deserialize, Clone, Copy, Debug, Default, PartialEq,
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
