use std::{collections::HashMap, fs::OpenOptions, hash::Hasher};

use anyhow::anyhow;
use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumProperty};
use uuid::Uuid;

use crate::picking::PickingMode;

use super::{Environment, Rarity};
use lib::worldgen::{asset::PortalDirection, brush::TerrainBrushRequest, voxel::VoxelMaterial};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Room {
    pub environment: Environment,
    pub rarity: Rarity,
    pub parts: HashMap<Uuid, RoomPart>,
}

impl Default for Room {
    fn default() -> Self {
        Self {
            environment: Environment::Development,
            rarity: Rarity::Uncommon,
            parts: Default::default(),
        }
    }
}

impl Room {
    pub fn push(&mut self, part: RoomPart) {
        self.parts.insert(part.uuid, part);
    }
}

#[derive(Component)]
pub struct RoomPartUuid(pub Uuid, pub Option<u64>);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RoomPart {
    pub uuid: Uuid,
    pub transform: Transform,
    pub data: RoomPartPayload,

    #[serde(skip_serializing, skip_deserializing)]
    pub place_after_spawn: bool,
}

#[derive(EnumProperty, EnumIter, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RoomPartPayload {
    #[strum(props(name = "STL Import"))]
    Stl {
        path: String,
        material: VoxelMaterial,
        vertices: Vec<[f32; 3]>,
        indices: Vec<u32>,
        geometry_hash: u64,
        vhacd_parameters: VhacdParameters,
    },
    #[strum(props(name = "Portal"))]
    Portal { direction: PortalDirection },
}

impl RoomPart {
    pub fn to_brush_request(&self) -> Option<TerrainBrushRequest> {
        let Self {
            uuid,
            transform,
            data,
            ..
        } = self;

        match data {
            RoomPartPayload::Stl {
                material,
                vertices,
                indices,
                vhacd_parameters,
                ..
            } => Some(TerrainBrushRequest::Mesh {
                uuid: (*uuid).into(),
                material: *material,
                transform: *transform,
                mesh: Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::MAIN_WORLD,
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone())
                .with_inserted_indices(Indices::U32(indices.clone())),
                vhacd_parameters: vhacd_parameters.clone(),
            }),
            _ => None,
        }
    }

    /// Determines how the room part should be placed after spawning it in the editor.
    pub fn placement(&self) -> Vec<PickingMode> {
        match self.data {
            RoomPartPayload::Stl { .. } => vec![PickingMode::GroundPlane],
            RoomPartPayload::Portal { .. } => {
                vec![PickingMode::Selectable, PickingMode::GroundPlane]
            }
        }
    }

    //
    // Stl
    //

    pub fn stl(path: &str, material: VoxelMaterial, transform: Transform) -> anyhow::Result<Self> {
        let (vertices, indices) = load_stl_to_raw_geometry(path)?;
        let vhacd_parameters = VhacdParameters::default();
        let geometry_hash = hash_geometry(&vertices, &indices, &vhacd_parameters);

        Ok(Self {
            uuid: Uuid::new_v4(),
            transform,
            data: RoomPartPayload::Stl {
                path: path.to_owned(),
                material,
                vertices,
                indices,
                geometry_hash,
                vhacd_parameters,
            },
            place_after_spawn: false,
        })
    }

    pub fn default_stl(transform: Transform) -> anyhow::Result<Self> {
        Self::stl(
            "assets/stl/default.stl",
            VoxelMaterial::BrownRock,
            transform,
        )
    }

    pub fn reload_stl(&mut self) -> anyhow::Result<()> {
        let RoomPartPayload::Stl {
            ref mut vertices,
            ref mut indices,
            ref mut geometry_hash,
            ref vhacd_parameters,
            path,
            ..
        } = &mut self.data
        else {
            return Err(anyhow!("not an stl"));
        };

        (*vertices, *indices) = load_stl_to_raw_geometry(&path)?;
        *geometry_hash = hash_geometry(&vertices, &indices, &vhacd_parameters);

        Ok(())
    }

    pub fn rehash_stl(&mut self) -> anyhow::Result<()> {
        let RoomPartPayload::Stl {
            ref vertices,
            ref indices,
            ref mut geometry_hash,
            ref vhacd_parameters,
            ..
        } = &mut self.data
        else {
            return Err(anyhow!("not an stl"));
        };

        *geometry_hash = hash_geometry(&vertices, &indices, &vhacd_parameters);

        Ok(())
    }

    //
    // Portal
    //

    pub fn portal(transform: Transform, direction: PortalDirection) -> Self {
        Self {
            uuid: Uuid::new_v4(),
            transform,
            data: RoomPartPayload::Portal { direction },
            place_after_spawn: false,
        }
    }
}

//
// Utility
//

fn hash_geometry(vertices: &[[f32; 3]], indices: &[u32], vhacd: &VhacdParameters) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();

    vertices
        .iter()
        .for_each(|v| v.iter().for_each(|f| hasher.write_u32(f.to_bits())));
    indices.iter().for_each(|i| hasher.write_u32(*i));

    hasher.write_u32(vhacd.concavity.to_bits());
    hasher.write_u32(vhacd.alpha.to_bits());
    hasher.write_u32(vhacd.beta.to_bits());
    hasher.write_u32(vhacd.resolution);
    hasher.write_u32(vhacd.plane_downsampling);
    hasher.write_u32(vhacd.convex_hull_downsampling);
    hasher.write_u8(match vhacd.fill_mode {
        FillMode::SurfaceOnly => 2,
        FillMode::FloodFill { detect_cavities } => match detect_cavities {
            true => 1,
            false => 0,
        },
    });
    hasher.write_u8(match vhacd.convex_hull_approximation {
        true => 1,
        false => 0,
    });
    hasher.write_u32(vhacd.max_convex_hulls);

    hasher.finish()
}

fn load_stl_to_raw_geometry(path: &str) -> anyhow::Result<(Vec<[f32; 3]>, Vec<u32>)> {
    let mut file = OpenOptions::new().read(true).open(path)?;
    let stl = stl_io::read_stl(&mut file)?;
    let stl_to_bevy_transform = Transform::from_rotation(Quat::from_euler(
        EulerRot::XZY,
        -90.0_f32.to_radians(),
        180.0_f32.to_radians(),
        0.0,
    ));

    let vertices = stl
        .vertices
        .into_iter()
        .map(|v| {
            // Transform to Y up / Z forward here so we don't
            // need to do it every time we export from Blender.
            let v: [f32; 3] = v.into();
            stl_to_bevy_transform.transform_point(v.into()).into()
        })
        .collect();
    let indices = stl
        .faces
        .into_iter()
        .flat_map(|f| {
            [
                f.vertices[0] as u32,
                f.vertices[1] as u32,
                f.vertices[2] as u32,
            ]
        })
        .collect();

    Ok((vertices, indices))
}
