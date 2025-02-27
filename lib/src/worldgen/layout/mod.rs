use std::{f32::consts::PI, fs::File, io::Read};

use avian3d::prelude::{Collider, Collision};
use bevy::{
    ecs::{system::SystemState, world::CommandQueue},
    prelude::*,
};
use bevy_rand::{
    global::GlobalEntropy,
    prelude::{Entropy, WyRand},
    traits::ForkableRng,
};
use consts::{ROOM_SHYNESS, SEQUENCE_DISTANCE};
use rand::Rng;
use room::{Portal, Room, SpawnRoomCommand};
use tunnel::{connect_portals, LayoutTrigger, PortalConnection};
use utility::{arrange_by_depenetration, Arrangement};

use crate::player::IsPlayer;

use super::asset::{AssetCollection, PortalDirection, RoomFlags};

mod consts;
mod room;
mod tunnel;
mod utility;
pub use room::Spawnpoint;

#[derive(Resource)]
pub struct LayoutState {
    pub rng: Entropy<WyRand>,
    pub sequence: usize,
}

pub struct InitLayoutCommand {
    pub after: CommandQueue,
}
pub struct StepLayoutCommand;

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (load_asset_collection, setup_state).chain());
        app.add_systems(Update, (debug, connect_portals, triggers));
    }
}

fn load_asset_collection(mut commands: Commands) {
    let path = if cfg!(debug_assertions) {
        "./assets/worldgen.staging.cbor"
    } else {
        "./assets/worldgen.production.cbor"
    };

    let mut file = File::open(path).expect("worldgen asset collection does not exist");
    let mut vec = Vec::new();
    file.read_to_end(&mut vec)
        .expect("failed to read worldgen asset collection");
    let assets: AssetCollection =
        cbor4ii::serde::from_slice(&vec).expect("failed to deserialize worldgen asset collection");

    commands.insert_resource(assets);
}

pub fn setup_state(mut commands: Commands, mut rng: GlobalEntropy<WyRand>) {
    commands.insert_resource(LayoutState {
        rng: rng.fork_rng(),
        sequence: 0,
    });
}

fn debug(
    mut gizmos: Gizmos,
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    portals: Query<(&Portal, &GlobalTransform)>,
) {
    if keyboard.just_released(KeyCode::KeyN) {
        commands.queue(StepLayoutCommand);
    }

    portals.iter().for_each(|portal| {
        let color = match portal.0.direction {
            PortalDirection::Entrance => Color::srgb(0.0, 0.0, 1.0),
            PortalDirection::Exit => Color::srgb(1.0, 0.0, 0.0),
            PortalDirection::Bidirectional => Color::srgb(0.0, 1.0, 0.0),
        };
        gizmos.sphere(
            Isometry3d {
                translation: portal.1.translation().into(),
                rotation: portal.1.rotation(),
            },
            3.0,
            color,
        );
    });
}

fn triggers(
    mut commands: Commands,
    mut collision_event_reader: EventReader<Collision>,
    state: Res<LayoutState>,
    player: Query<&IsPlayer>,
    trigger: Query<(&Parent, &LayoutTrigger)>,
    connections: Query<(Entity, &PortalConnection)>,
    portals: Query<(&Parent, &Portal)>,
    rooms: Query<(Entity, &Room)>,
) {
    for Collision(contacts) in collision_event_reader.read() {
        if player.get(contacts.entity1).is_err() && player.get(contacts.entity2).is_err() {
            continue;
        }
        let (trigger_entity, (trigger_parent, trigger)) = {
            if let Ok(trigger) = trigger.get(contacts.entity1) {
                (contacts.entity1, trigger)
            } else if let Ok(trigger) = trigger.get(contacts.entity2) {
                commands.entity(contacts.entity2).despawn();
                (contacts.entity2, trigger)
            } else {
                continue;
            }
        };

        let Ok((connection_entity, connection)) = connections.get(**trigger_parent) else {
            continue;
        };

        match trigger {
            LayoutTrigger::GenerateNextSequence => {
                println!("{} {}", connection.sequence, state.sequence);
                if connection.sequence == state.sequence {
                    //commands.queue(StepLayoutCommand);
                }
            }
            LayoutTrigger::UnloadPreviousSequence => {
                let Ok((to_portal_parent, _)) = portals.get(connection.to_portal) else {
                    continue;
                };
                let Ok(_) = rooms.get(**to_portal_parent) else {
                    continue;
                };
                let Ok((from_portal_parent, _)) = portals.get(connection.from_portal) else {
                    continue;
                };
                let Ok(from_room) = rooms.get(**from_portal_parent) else {
                    continue;
                };

                let mut entity_distances = vec![(connection_entity, 0)];
                walk_room(
                    &mut entity_distances,
                    &connections,
                    &portals,
                    &rooms,
                    from_room,
                    -1,
                );

                for (entity, distance) in entity_distances.into_iter() {
                    if distance < 0 {
                        commands.entity(entity).despawn_recursive();
                    }
                }

                commands.queue(StepLayoutCommand);
            }
        }

        commands.entity(trigger_entity).despawn();
    }
}

fn walk_room(
    entity_distances: &mut Vec<(Entity, isize)>,
    connections: &Query<(Entity, &PortalConnection)>,
    portals: &Query<(&Parent, &Portal)>,
    rooms: &Query<(Entity, &Room)>,
    room: (Entity, &Room),
    depth: isize,
) {
    entity_distances.push((room.0, depth));

    for portal_entity in room.1.portals.iter() {
        let Ok(portal) = portals.get(*portal_entity) else {
            continue;
        };

        let Some(connection_entity) = portal.1.connection else {
            continue;
        };
        if entity_distances
            .iter()
            .any(|(entity, _)| *entity == connection_entity)
        {
            continue;
        }
        let Ok(connection) = connections.get(connection_entity) else {
            continue;
        };

        walk_connection(
            entity_distances,
            connections,
            portals,
            rooms,
            connection,
            depth + if depth > 0 { 1 } else { -1 },
        );
    }
}

fn walk_connection(
    entity_distances: &mut Vec<(Entity, isize)>,
    connections: &Query<(Entity, &PortalConnection)>,
    portals: &Query<(&Parent, &Portal)>,
    rooms: &Query<(Entity, &Room)>,
    connection: (Entity, &PortalConnection),
    depth: isize,
) {
    entity_distances.push((connection.0, depth));

    for portal_entity in vec![connection.1.from_portal, connection.1.to_portal].into_iter() {
        let Ok((portal_parent, _)) = portals.get(portal_entity) else {
            continue;
        };
        if entity_distances
            .iter()
            .any(|(entity, _)| *entity == **portal_parent)
        {
            continue;
        }
        let Ok(room) = rooms.get(**portal_parent) else {
            continue;
        };

        walk_room(
            entity_distances,
            connections,
            portals,
            rooms,
            room,
            depth + if depth > 0 { 1 } else { -1 },
        );
    }
}

impl Command for InitLayoutCommand {
    fn apply(mut self, world: &mut World) {
        let mut system_state: SystemState<(Commands, ResMut<LayoutState>, Res<AssetCollection>)> =
            SystemState::new(world);
        let (mut commands, mut state, assets) = system_state.get_mut(world);

        if state.sequence != 0 {
            panic!("layout is already initialized");
        }

        let room = assets
            .random_room_with_flags(RoomFlags::Spawnable, &mut state.rng)
            .clone();
        commands.queue(SpawnRoomCommand {
            sequence: 0,
            arrangement: Arrangement {
                spherical: true,
                collider: Collider::sphere(room.radius() + ROOM_SHYNESS),
                position: (state.rng.gen::<Vec3>() - Vec3::splat(0.5)).into(),
                rotation: Quat::from_euler(
                    EulerRot::YXZ,
                    state.rng.gen_range(0.0..(PI * 2.0)),
                    0.0,
                    0.0,
                )
                .into(),
            },
            room,
            connect_to_portals: default(),
        });

        commands.append(&mut self.after);
        system_state.apply(world);
    }
}

impl Command for StepLayoutCommand {
    fn apply(self, world: &mut World) {
        let mut system_state: SystemState<(
            Commands,
            ResMut<LayoutState>,
            Res<AssetCollection>,
            Query<&Arrangement>,
            Query<(&Room, &GlobalTransform)>,
            Query<(&Portal, Entity, &GlobalTransform)>,
        )> = SystemState::new(world);
        let (mut commands, mut state, assets, arrangeables, rooms, portals) =
            system_state.get_mut(world);

        // Find available exit portals from the previous sequence.
        let prev_rooms = rooms
            .iter()
            .filter(|room| room.0.sequence == state.sequence)
            .collect::<Vec<_>>();
        let prev_portal_entities = prev_rooms
            .iter()
            .flat_map(|room| room.0.portals.clone())
            .collect::<Vec<_>>();
        let mut prev_portals = prev_portal_entities
            .into_iter()
            .filter_map(|portal| {
                if let Ok(portal) = portals.get(portal) {
                    if portal.0.connection.is_none() && portal.0.direction.is_exit() {
                        return Some(portal);
                    }
                }
                None
            })
            .collect::<Vec<_>>();

        state.sequence += 1;

        // Choose next rooms.
        let next_room_count = match prev_portals.len() {
            0 => panic!("no unconnected exits"),
            1 => 1,
            _ => state.rng.gen_range(1..=prev_portals.len()),
        };
        let next_rooms = (0..next_room_count)
            .map(|_| assets.random_room(&mut state.rng).clone())
            .collect::<Vec<_>>();

        // Arrange next rooms.
        let prev_room_positions = rooms
            .iter()
            .filter_map(|room| {
                if room.0.sequence != state.sequence - 1 {
                    return None;
                }
                Some(room.1.translation())
            })
            .collect::<Vec<_>>();
        let avg_position =
            prev_room_positions.iter().sum::<Vec3>() / prev_room_positions.len() as f32;
        let bias_direction = avg_position.cross(Vec3::Y).normalize();
        let start_position = avg_position + bias_direction * SEQUENCE_DISTANCE;
        let mut next_room_arrangeables = next_rooms
            .iter()
            .map(|room| {
                let spherical = true;
                Arrangement {
                    spherical,
                    collider: Collider::sphere(room.radius() + ROOM_SHYNESS),
                    position: (start_position + state.rng.gen::<Vec3>() - Vec3::splat(0.5)).into(),
                    rotation: Quat::from_euler(
                        EulerRot::YXZ,
                        state.rng.gen_range(0.0..(2.0 * PI)),
                        0.0,
                        0.0,
                    )
                    .into(),
                }
            })
            .collect::<Vec<Arrangement>>();
        let static_arrangeables = arrangeables
            .iter()
            .map(|arrangeable| arrangeable.clone())
            .collect();
        arrange_by_depenetration(&mut next_room_arrangeables, static_arrangeables);

        next_rooms
            .into_iter()
            .zip(next_room_arrangeables)
            .for_each(|(room, arrangement)| {
                let exit_index = match prev_portals.len() {
                    0 => panic!("no unconnected exits"),
                    1 => 0,
                    _ => state.rng.gen_range(0..prev_portals.len()),
                };
                let from_portal = prev_portals.remove(exit_index);

                commands.queue(SpawnRoomCommand {
                    sequence: state.sequence,
                    arrangement,
                    room,
                    connect_to_portals: vec![from_portal.1],
                });
            });

        system_state.apply(world);
    }
}
