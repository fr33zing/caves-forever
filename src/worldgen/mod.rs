pub mod brush;
pub mod chunk;
pub mod layout;
pub mod terrain;
pub mod voxel;

pub mod consts {
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
    pub const CHUNK_RENDER_BORDERS: bool = false;
    pub const CHUNK_INTERNAL_GEOMETRY: bool = true;
    pub const WORLD_RENDER_ORIGIN: bool = false;
}
