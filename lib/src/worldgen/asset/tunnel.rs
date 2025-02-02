use nalgebra::Point2;
use serde::{Deserialize, Serialize};

// All tunnel profiles must have this number of points.
pub const TUNNEL_POINTS: usize = 16;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Tunnel {
    pub source: String,
    pub weight: f32,
    pub points: [Point2<f32>; TUNNEL_POINTS],
}
