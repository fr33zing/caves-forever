use bevy::prelude::*;

use super::{RangedMode, RangedSpread, Weapon, WeaponAction};

pub const SHOTGUN: Weapon = Weapon {
    name: "Shotgun",
    model: "models/weapon/shotgun.glb",
    action: WeaponAction::Ranged {
        spread: RangedSpread::Circle(10.0),
        mode: RangedMode::Hitscan,
        projectiles: 8,
    },
    viewmodel_offset: Vec3::new(0.175, -0.125, -0.4),
};
