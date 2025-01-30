use serde::{Deserialize, Serialize};

mod room;
mod tunnel;
pub use room::*;
pub use tunnel::*;

#[derive(Serialize, Deserialize, Debug)]
pub struct AssetCollection {
    tunnels: Vec<Tunnel>,
    rooms: Vec<Room>,
}
