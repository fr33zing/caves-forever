use avian3d::prelude::Collider;
use bevy::prelude::Transform;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Room {
    pub weight: f32,
    pub cavities: Vec<Collider>,
    pub portals: Vec<Transform>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Portal {
    transform: Transform,
    bidirectional: bool,
}
