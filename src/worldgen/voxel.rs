use bevy::prelude::*;
use noisy_bevy::simplex_noise_3d;
use serde::{Deserialize, Serialize};
use strum::EnumProperty;
use strum_macros::FromRepr;

#[derive(Clone, Copy, Debug)]
pub struct VoxelSample {
    pub material: VoxelMaterial,
    pub distance: f32,
}

#[derive(Debug)]
pub enum VoxelHardness {
    Default,
    Value(f32),
    Unbreakable,
}

impl VoxelHardness {
    pub fn multiplier(&self) -> f32 {
        match self {
            VoxelHardness::Default => 1.0,
            VoxelHardness::Value(h) => *h,
            VoxelHardness::Unbreakable => 10000.0,
        }
    }
}

#[derive(FromRepr, EnumProperty, Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
#[repr(u8)]
pub enum VoxelMaterial {
    #[strum(props(Name = "Unset"))]
    Unset = 255,

    #[strum(props(Name = "Invalid"))]
    Invalid = 254,

    #[strum(props(Name = "Boundary"))]
    Boundary = 253,

    #[strum(props(Name = "Fake Boundary"))]
    FakeBoundary = 252,

    #[strum(props(Name = "Brown Rock"))]
    BrownRock = 0,

    #[strum(props(Name = "Smooth Yellow Rock"))]
    YellowRock = 1,

    #[strum(props(Name = "Shiny Green Rock"))]
    ShinyGreenRock = 2,
}

impl VoxelMaterial {
    pub fn hardness(&self) -> VoxelHardness {
        match self {
            VoxelMaterial::Boundary => VoxelHardness::Unbreakable,
            VoxelMaterial::FakeBoundary => VoxelHardness::Value(5.0),
            VoxelMaterial::BrownRock => VoxelHardness::Value(1.5),
            VoxelMaterial::ShinyGreenRock => VoxelHardness::Value(4.0),
            _ => VoxelHardness::Default,
        }
    }

    pub fn sdf_noise(&self, point: &Vec3, distance: &f32) -> f32 {
        let external = *distance >= 0.0;
        let mut noise = 0.0;

        match self {
            VoxelMaterial::BrownRock => {
                if external {
                    noise += simplex_noise_3d(point / 2.0) * 0.25;
                    noise += simplex_noise_3d(point / 4.0) * 0.25;
                    noise += simplex_noise_3d(point / 8.0) * 0.25;
                    noise += simplex_noise_3d(point / Vec3::new(64.0, 6.0, 64.0)) * 0.75;
                }
            }
            VoxelMaterial::YellowRock => {
                noise += simplex_noise_3d(*point) * 0.25;
                noise += simplex_noise_3d(point / 5.0) * 0.25;
            }

            _ => {}
        };

        noise
    }
}
