pub mod asset;
pub mod brush;
pub mod chunk;
pub mod layout;
pub mod terrain;
pub mod voxel;

pub mod consts {
    use avian3d::prelude::{FillMode, VhacdParameters};

    pub const CHUNK_SIZE: u32 = 32;

    pub const CHUNK_SAMPLE_RESOLUTION: f32 = 1.0 / 1.0; // RHS must be a power of 2

    pub const CHUNK_SAMPLE_SIZE: u32 = (CHUNK_SIZE_F * CHUNK_SAMPLE_RESOLUTION) as u32;
    pub const VOXEL_REAL_SIZE: f32 = (CHUNK_SIZE / CHUNK_SAMPLE_SIZE) as f32;

    pub const CHUNK_SIZE_F: f32 = CHUNK_SIZE as f32;
    pub const CHUNK_SAMPLE_SIZE_F: f32 = CHUNK_SAMPLE_SIZE as f32;

    // For debugging only
    pub const CHUNK_RENDER_BORDERS: bool = true;
    pub const CHUNK_INTERNAL_GEOMETRY: bool = true;
    pub const WORLD_RENDER_ORIGIN: bool = false;

    pub const TUNNEL_VHACD_PARAMETERS: VhacdParameters = VhacdParameters {
        // Changed
        alpha: 0.025,
        beta: 0.025,
        // Default
        resolution: 48,
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

pub mod utility {
    use anyhow::anyhow;
    use avian3d::prelude::{Collider, VhacdParameters};
    use bevy::prelude::Mesh;
    use std::sync::Mutex;

    pub fn safe_vhacd(mesh: &Mesh, vhacd_parameters: &VhacdParameters) -> anyhow::Result<Collider> {
        let mesh = Mutex::new(mesh);
        std::panic::catch_unwind(|| {
            let collider = Collider::convex_decomposition_from_mesh_with_config(
                &mesh.lock().unwrap(),
                vhacd_parameters,
            );
            collider
        })
        .map_err(|_| anyhow!("convex decomposition panicked"))?
        .ok_or_else(|| anyhow!("convex decomposition failed"))
    }
}
