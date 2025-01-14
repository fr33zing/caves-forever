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

#[derive(FromRepr, EnumProperty, Debug, PartialEq, Eq, Clone, Copy)]
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
            VoxelMaterial::ShinyGreenRock => VoxelHardness::Value(4.0),
            _ => VoxelHardness::Default,
        }
    }
}
