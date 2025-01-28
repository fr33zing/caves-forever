use std::{collections::HashMap, fs::OpenOptions};

use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::worldgen::{brush::TerrainBrushRequest, voxel::VoxelMaterial};

use super::{Environment, Rarity};

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
    pub fn push(&mut self, part: RoomPart) -> Uuid {
        let uuid = Uuid::new_v4();
        self.parts.insert(uuid, part);
        uuid
    }
}

#[derive(Component)]
pub struct RoomPartUuid(pub Uuid);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RoomPart {
    pub transform: Transform,
    pub data: RoomPartPayload,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum RoomPartPayload {
    Stl {
        path: String,
        material: VoxelMaterial,
        vertices: Vec<[f32; 3]>,
        indices: Vec<u32>,
    },
}

impl RoomPart {
    pub fn to_brush_request(&self) -> TerrainBrushRequest {
        let Self { transform, data } = self;

        match data {
            RoomPartPayload::Stl {
                material,
                vertices,
                indices,
                ..
            } => TerrainBrushRequest::Mesh {
                material: *material,
                transform: *transform,
                mesh: Mesh::new(
                    PrimitiveTopology::TriangleList,
                    RenderAssetUsages::MAIN_WORLD,
                )
                .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone())
                .with_inserted_indices(Indices::U32(indices.clone())),
            },
        }
    }

    pub fn stl(path: &str, material: VoxelMaterial, transform: Transform) -> anyhow::Result<Self> {
        let mut file = OpenOptions::new().read(true).open(path)?;
        let stl = stl_io::read_stl(&mut file)?;
        let stl_to_bevy_transform = Transform::from_rotation(Quat::from_euler(
            EulerRot::XZY,
            -90.0_f32.to_radians(),
            180.0_f32.to_radians(),
            0.0,
        ));

        Ok(Self {
            transform,
            data: RoomPartPayload::Stl {
                path: path.to_owned(),
                material,
                vertices: stl
                    .vertices
                    .into_iter()
                    .map(|v| {
                        // Transform to Y up / Z forward here so we don't
                        // need to do it every time we export from Blender.
                        let v: [f32; 3] = v.into();
                        stl_to_bevy_transform.transform_point(v.into()).into()
                    })
                    .collect(),
                indices: stl
                    .faces
                    .into_iter()
                    .flat_map(|f| {
                        [
                            f.vertices[0] as u32,
                            f.vertices[1] as u32,
                            f.vertices[2] as u32,
                        ]
                    })
                    .collect(),
            },
        })
    }

    pub fn default_stl(transform: Transform) -> anyhow::Result<Self> {
        Self::stl(
            "assets/stl/default.stl",
            VoxelMaterial::BrownRock,
            transform,
        )
    }
}
