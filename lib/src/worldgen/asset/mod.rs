use bevy::prelude::Resource;
use bevy_rand::prelude::*;
use rand::prelude::*;
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

impl AssetCollection {
    pub fn random_tunnel<R>(&self, rng: &mut R) -> &Tunnel
    where
        R: Rng + ?Sized,
    {
        self.tunnels
            .choose_weighted(rng, |tunnel| tunnel.weight)
            .unwrap()
    }

    pub fn random_room(&self, rng: &mut Entropy<WyRand>) -> &Room {
        self.rooms.choose_weighted(rng, |room| room.weight).unwrap()
    }

    pub fn random_room_with_flags<R>(&self, flags: RoomFlags, rng: &mut R) -> &Room
    where
        R: Rng + ?Sized,
    {
        let rooms = self
            .rooms
            .iter()
            .filter(|room| room.flags.contains(flags.clone()))
            .collect::<Vec<_>>();

        rooms.choose_weighted(rng, |room| room.weight).unwrap()
    }
}
