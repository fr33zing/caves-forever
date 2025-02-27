use bevy::{ecs::system::SystemState, prelude::*};
use rand::Rng;

use crate::worldgen::{
    asset::{self, PortalDirection},
    brush::TerrainBrush,
    voxel::VoxelMaterial,
};

use super::{tunnel::PendingPortalConnection, utility::Arrangement, LayoutState};

#[derive(Component)]
pub struct Room {
    pub sequence: usize,
    pub portals: Vec<Entity>,
    pub radius: f32,
}

#[derive(Component)]
pub struct Portal {
    pub direction: PortalDirection,
    pub connection: Option<Entity>,
}
impl Portal {
    pub fn inward(&self, transform: &GlobalTransform) -> Vec3 {
        if self.direction == PortalDirection::Entrance {
            return *transform.up();
        }
        -*transform.up()
    }
}

#[derive(Component)]
pub struct Spawnpoint;

pub struct SpawnRoomCommand {
    pub sequence: usize,
    pub arrangement: Arrangement,
    pub room: asset::Room,
    pub connect_to_portals: Vec<Entity>,
}

fn position_and_angle_transform(position: Vec3, angle: f32) -> Transform {
    let rotation = Quat::from_euler(EulerRot::YXZ, angle, 0.0, 0.0);
    Transform::from_translation(position).with_rotation(rotation)
}

impl Command for SpawnRoomCommand {
    fn apply(self, world: &mut World) {
        let mut system_state: SystemState<(Commands, ResMut<LayoutState>)> =
            SystemState::new(world);
        let (mut commands, mut state) = system_state.get_mut(world);

        let mut transform = self.arrangement.transform();
        transform.translation += self.room.inverse_world_origin_offset();

        let mut room = Room {
            sequence: self.sequence,
            portals: default(),
            radius: self.room.radius(),
        };

        commands
            .spawn(transform)
            .with_children(|parent| {
                // Arrangement
                parent.spawn(self.arrangement);

                // Cavities
                self.room.cavities.iter().for_each(|cavity| {
                    parent.spawn(TerrainBrush::collider(
                        "",
                        self.sequence,
                        VoxelMaterial::Invalid,
                        cavity.clone(),
                        transform,
                    ));
                });

                // Portals
                room.portals = self
                    .room
                    .portals
                    .iter()
                    .map(|portal| {
                        parent
                            .spawn((
                                portal.transform,
                                Portal {
                                    direction: portal.direction,
                                    connection: None,
                                },
                            ))
                            .id()
                    })
                    .collect();

                // Pending connections
                let mut entrances = room
                    .portals
                    .iter_mut()
                    .zip(self.room.portals)
                    .filter(|(_, portal)| portal.direction.is_entrance())
                    .map(|(entity, _)| entity.clone())
                    .collect::<Vec<_>>();
                self.connect_to_portals.into_iter().for_each(|from_portal| {
                    let entrance_index = match entrances.len() {
                        0 => panic!("no unconnected entrances"),
                        1 => 0,
                        _ => state.rng.gen_range(0..entrances.len()),
                    };
                    let to_portal = entrances.remove(entrance_index);

                    parent.spawn(PendingPortalConnection {
                        sequence: self.sequence,
                        from_portal,
                        to_portal,
                    });
                });

                // Spawnpoints
                self.room.spawnpoints.iter().for_each(|spawnpoint| {
                    parent.spawn((
                        position_and_angle_transform(spawnpoint.position, spawnpoint.angle),
                        Spawnpoint,
                    ));
                });
            })
            .insert(room);

        system_state.apply(world);
    }
}
