use avian3d::prelude::Collider;
use bevy::prelude::Transform;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Room {
    pub weight: f32,
    pub cavity: Collider,
    pub portals: Vec<Transform>,
}
