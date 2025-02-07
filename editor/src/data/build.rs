use std::collections::HashMap;

use anyhow::anyhow;
use avian3d::prelude::{Position, Rotation};
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use strum::IntoEnumIterator;

use super::{Room, RoomPart, RoomPartPayload, Tunnel};
use lib::worldgen::{
    asset::{self, PortalDirection, RoomFlags, Spawnpoint},
    utility::safe_vhacd,
};

impl Tunnel {
    pub fn build(&self, source: String) -> anyhow::Result<asset::Tunnel> {
        Ok(asset::Tunnel {
            source,
            weight: self.rarity.weight(),
            points: self.points,
        })
    }
}

impl Room {
    pub fn build(&self, source: String) -> anyhow::Result<asset::Room> {
        let mut room = asset::Room::new(self.rarity.weight(), source)?;

        // TODO adjust transform so everything is centered on world origin
        // each roompart must implement compute_aabb()

        for part in self.parts.values().cloned() {
            let RoomPart {
                transform, data, ..
            } = part;

            match data {
                RoomPartPayload::Stl {
                    vertices,
                    indices,
                    vhacd_parameters,
                    ..
                } => {
                    let mesh = Mesh::new(
                        PrimitiveTopology::TriangleList,
                        RenderAssetUsages::MAIN_WORLD,
                    )
                    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vertices.clone())
                    .with_inserted_indices(Indices::U32(indices.clone()))
                    .transformed_by(transform);

                    let collider = safe_vhacd(&mesh, &vhacd_parameters)?;
                    room.cavities.push(collider);
                }
                RoomPartPayload::Portal { direction } => {
                    room.portals.push(asset::Portal {
                        transform,
                        direction,
                    });
                }
                RoomPartPayload::Spawnpoint => {
                    room.flags |= RoomFlags::Spawnable;
                    room.spawnpoints.push(Spawnpoint {
                        position: transform.translation,
                        // TODO make sure this is right
                        angle: transform.rotation.to_euler(EulerRot::YXZ).0,
                    })
                }
            }
        }

        let problems = validate(&room);
        if problems.len() > 0 {
            let problems = problems
                .into_iter()
                .map(|p| format!("- {p}"))
                .collect::<Vec<_>>()
                .join("\n");

            return Err(anyhow!(problems));
        }

        Ok(room)
    }
}

fn validate(
    asset::Room {
        cavities,
        portals,
        spawnpoints,
        ..
    }: &asset::Room,
) -> Vec<String> {
    let mut problems = Vec::<String>::new();

    // Cavities
    if cavities.len() == 0 {
        problems.push("no cavities".into());
    }

    // Portals
    let mut valid_portals = PortalDirection::iter()
        .map(|d| (d, 0))
        .collect::<HashMap<_, u8>>();

    for (i, portal) in portals.iter().enumerate() {
        let mut direction_problem = |s: &str| {
            problems.push(format!(
                "portal [{i}] direction is {} but {s}",
                portal.direction
            ));
        };

        let test_points = [
            portal.transform.transform_point(Vec3::Y / 2.0), // Inward
            portal.transform.transform_point(Vec3::NEG_Y / 2.0), // Outward
        ];
        let mut inside = (false, false);

        for cavity in cavities {
            let inside_this = test_points
                .into_iter()
                .map(|point| {
                    cavity
                        .project_point(Position::default(), Rotation::default(), point, true)
                        .1
                })
                .collect::<Vec<_>>();

            inside.0 |= inside_this[0];
            inside.1 |= inside_this[1];

            if inside.0 && inside.1 {
                break;
            }
        }

        match (portal.direction, inside.0, inside.1) {
            (PortalDirection::Entrance, true, true)
            | (PortalDirection::Exit, true, true)
            | (PortalDirection::Bidirectional, true, true) => {
                direction_problem("both faces are internal")
            }
            (PortalDirection::Entrance, false, false)
            | (PortalDirection::Exit, false, false)
            | (PortalDirection::Bidirectional, false, false) => {
                direction_problem("both faces are external")
            }
            (PortalDirection::Entrance, false, true) => direction_problem("it points outward"),
            (PortalDirection::Exit, true, false) => direction_problem("it points inward"),
            _ => {
                *valid_portals.get_mut(&portal.direction).unwrap() += 1;
            }
        }
    }

    let entrances = *valid_portals.get(&PortalDirection::Entrance).unwrap();
    let exits = *valid_portals.get(&PortalDirection::Exit).unwrap();
    let bidirectionals = *valid_portals.get(&PortalDirection::Bidirectional).unwrap();

    if entrances == 0 && exits == 0 && bidirectionals < 2 {
        problems.push("no valid entrance or exit".into());
    } else if entrances == 0 && exits == 1 && bidirectionals == 0 {
        problems.push("no valid entrance".into());
    } else if entrances == 1 && exits == 0 && bidirectionals == 0 {
        problems.push("no valid exit".into());
    }

    // Spawnpoints
    let out_of_bounds_spawnpoints = spawnpoints.iter().any(|spawnpoint| {
        !cavities.iter().any(|cavity| {
            cavity.contains_point(
                Position::default(),
                Rotation::default(),
                spawnpoint.position,
            )
        })
    });
    if out_of_bounds_spawnpoints {
        problems.push("out-of-bounds spawnpoint(s)".into());
    }

    problems
}
