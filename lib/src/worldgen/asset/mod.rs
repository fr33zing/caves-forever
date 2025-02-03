use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

mod room;
mod tunnel;
pub use room::*;
pub use tunnel::*;

#[derive(Serialize, Deserialize, Debug, Default, Resource)]
pub struct AssetCollection {
    pub tunnels: Vec<Tunnel>,
    pub rooms: Vec<Room>,
}
