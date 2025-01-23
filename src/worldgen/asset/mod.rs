use serde::{Deserialize, Serialize};
use strum::EnumIter;

mod tunnel;
pub use tunnel::*;

#[repr(u8)]
#[derive(
    EnumIter, strum_macros::Display, Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq,
)]
pub enum Environment {
    /// Asset will be used in release mode and debug mode.
    Production = 0,

    /// Asset will be used in debug mode.
    Staging = 1,

    /// Asset will not be used.
    Development = 2,
}

#[repr(u8)]
#[derive(
    EnumIter, strum_macros::Display, Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq,
)]
pub enum Rarity {
    Abundant = 0,
    Common = 1,
    Uncommon = 3,
    Rare = 4,
    Exotic = 5,
}

impl Rarity {
    pub fn weight(&self) -> f32 {
        match self {
            Rarity::Abundant => 3.0,
            Rarity::Common => 2.0,
            Rarity::Uncommon => 1.0,
            Rarity::Rare => 0.5,
            Rarity::Exotic => 0.3,
        }
    }
}
