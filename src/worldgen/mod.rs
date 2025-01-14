pub mod brush;
pub mod layout;
pub mod terrain;
mod voxel_materials;

pub use voxel_materials::{VoxelHardness, VoxelMaterial};

pub mod consts {
    pub const CHUNK_RENDER_BORDERS: bool = true;
    pub const CHUNK_FLAT_NORMALS: bool = true;
    pub const CHUNK_INTERNAL_GEOMETRY: bool = true;

    pub const CHUNK_SIZE: u32 = 32;
    pub const CHUNK_SAMPLE_RESOLUTION: f32 = 1.0 / 2.0; // RHS must be a power of 2
    pub const CHUNK_SAMPLE_SIZE: u32 = (CHUNK_SIZE as f32 * CHUNK_SAMPLE_RESOLUTION) as u32;
    pub const VOXEL_REAL_SIZE: f32 = (CHUNK_SIZE / CHUNK_SAMPLE_SIZE) as f32;
}
