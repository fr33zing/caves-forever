pub mod asset;
pub mod brush;
pub mod chunk;
pub mod layout;
pub mod terrain;
pub mod voxel;

pub mod consts {
    use avian3d::prelude::{FillMode, VhacdParameters};

    pub const CHUNK_SIZE: u32 = 32;

    #[cfg(not(feature = "webgl2"))]
    pub const CHUNK_SAMPLE_RESOLUTION: f32 = 1.0 / 1.0; // RHS must be a power of 2
    #[cfg(feature = "webgl2")]
    pub const CHUNK_SAMPLE_RESOLUTION: f32 = 1.0 / 4.0;

    pub const CHUNK_SAMPLE_SIZE: u32 = (CHUNK_SIZE_F * CHUNK_SAMPLE_RESOLUTION) as u32;
    pub const VOXEL_REAL_SIZE: f32 = (CHUNK_SIZE / CHUNK_SAMPLE_SIZE) as f32;

    pub const CHUNK_SIZE_F: f32 = CHUNK_SIZE as f32;
    pub const CHUNK_SAMPLE_SIZE_F: f32 = CHUNK_SAMPLE_SIZE as f32;

    // For debugging only
    pub const CHUNK_RENDER_BORDERS: bool = true;
    pub const CHUNK_INTERNAL_GEOMETRY: bool = true;
    pub const WORLD_RENDER_ORIGIN: bool = false;

    pub const VHACD_PARAMETERS: VhacdParameters = VhacdParameters {
        // Changed
        alpha: 0.025,
        beta: 0.025,
        // Default
        resolution: 64,
        concavity: 0.01,
        plane_downsampling: 4,
        convex_hull_downsampling: 4,
        convex_hull_approximation: true,
        max_convex_hulls: 1024,
        fill_mode: FillMode::FloodFill {
            detect_cavities: false,
        },
    };
}
