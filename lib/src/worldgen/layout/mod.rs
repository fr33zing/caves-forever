use std::{f32::consts::PI, fs::File, io::Read};

use bevy::{ecs::system::SystemState, prelude::*};
use bevy_rand::{global::GlobalEntropy, prelude::*, traits::ForkableRng};
use pathfinding::prelude::dijkstra;
use rand::{seq::SliceRandom, Rng};
use uuid::Uuid;

mod utility;
use utility::*;

use super::{
    asset::{AssetCollection, PortalDirection, Room},
    brush::TerrainBrush,
    voxel::VoxelMaterial,
};

pub const SEQUENCE_DISTANCE: f32 = 128.0;
pub const NODE_ARRANGEMENT_RADIUS_INFLATE: f32 = 32.0;
pub const EDGE_PATHING_RADIUS_INFLATE: f32 = 24.0;
pub const HULL_DENSITY: f32 = 0.0001;

#[derive(Component)]
pub struct LayoutSequence(pub usize);

/// A tunnel.
#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub sequence: usize,
    pub from_node: usize,
    pub from_portal: usize,
    pub to_node: usize,
    pub to_portal: usize,
}

/// A room.
#[derive(Debug, Clone, Component)]
pub struct LayoutNode {
    pub sequence: usize,
    /// Index of the node within its sequence.
    pub index: usize,

    pub position: Vec3,
    pub angle: f32,
    pub radius: f32,
    pub room: Room,
}

#[derive(Debug, Resource)]
pub struct LayoutGraph {
    pub rng: Entropy<WyRand>,
    pub sequence: usize,
    pub nodes: Vec<Vec<LayoutNode>>,
    pub edges: Vec<LayoutEdge>,
    pub path_points: Vec<Vec3>,
}

pub struct StepLayoutCommand;

#[derive(Component)]
pub struct LayoutPathDebug {
    color: Color,
    points: Vec<IVec3>,
    path: Vec<IVec3>,
}

#[derive(Component)]
pub struct LayoutPortalDebug {
    position: Vec3,
    direction: PortalDirection,
}

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Startup,
            (
                (setup_state, setup_spawn_room).chain(),
                load_asset_collection,
            ),
        );
        app.add_systems(Update, debug);
    }
}

fn debug(
    mut gizmos: Gizmos,
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    paths: Query<&LayoutPathDebug>,
    portals: Query<&LayoutPortalDebug>,
) {
    if keyboard.just_released(KeyCode::KeyN) {
        commands.queue(StepLayoutCommand);
    }

    paths.iter().for_each(|p| {
        let color = p.color;
        p.points.iter().for_each(|p| {
            gizmos.sphere(Isometry3d::from_translation(p.as_vec3()), 0.1, color);
        });

        p.path.windows(2).for_each(|w| {
            gizmos.line(w[0].as_vec3(), w[1].as_vec3(), color);
        });
    });

    portals.iter().for_each(|p| {
        gizmos.sphere(
            Isometry3d::from_translation(p.position),
            2.0,
            match p.direction {
                PortalDirection::Entrance => Color::srgb(0.1, 0.1, 1.0),
                PortalDirection::Exit => Color::srgb(1.0, 0.1, 0.1),
                PortalDirection::Bidirectional => Color::srgb(0.1, 1.0, 0.1),
            },
        );
    });
}

fn setup_state(mut commands: Commands, mut rng: GlobalEntropy<WyRand>) {
    commands.insert_resource(LayoutGraph {
        rng: rng.fork_rng(),
        sequence: 0,
        nodes: default(),
        edges: default(),
        path_points: default(),
    });
}

fn setup_spawn_room(
    mut commands: Commands,
    mut graph: ResMut<LayoutGraph>,
    assets: Res<AssetCollection>,
) {
    let node = LayoutNode {
        sequence: 0,
        index: 0,
        position: Vec3::ONE,
        angle: 0.0,
        radius: 32.0,
        room: assets.random_room(&mut graph.rng).clone(),
    };
    commands
        .spawn(Transform::from_translation(Vec3::ZERO))
        .with_children(|parent| {
            for cavity in node.room.cavities.iter() {
                parent.spawn(TerrainBrush::collider(
                    &Uuid::new_v4().to_string(),
                    graph.sequence,
                    VoxelMaterial::Invalid,
                    cavity.clone(),
                    Transform::from_translation(node.position),
                ));
            }
        });
    graph.nodes.push(vec![node]);
    graph.sequence += 1;
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

impl LayoutNode {
    pub fn rotation(&self) -> Transform {
        Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, self.angle, 0.0, 0.0))
    }

    pub fn transform(&self) -> Transform {
        self.rotation().with_translation(self.position)
    }
}

impl LayoutGraph {
    pub fn node(&self, sequence: usize, index: usize) -> &LayoutNode {
        &self.nodes[self.sequence - sequence][index]
    }

    pub fn unconnected_portal_indices(&self, node: &LayoutNode, entrance: bool) -> Vec<usize> {
        node.room
            .portals
            .iter()
            .enumerate()
            .filter_map(|(portal_index, portal)| {
                // TODO this is probably incorrect
                let connected = self.edges.iter().any(|edge| {
                    (edge.sequence == node.sequence + 1
                        && edge.from_node == node.index
                        && edge.from_portal == portal_index)
                        || (edge.sequence == node.sequence
                            && edge.to_node == node.index
                            && edge.to_portal == portal_index)
                });
                let correct_direction = match portal.direction {
                    PortalDirection::Entrance => entrance,
                    PortalDirection::Exit => !entrance,
                    PortalDirection::Bidirectional => true,
                };
                if !connected && correct_direction {
                    Some(portal_index)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    }

    /// Returns (node_index, portal_index)
    pub fn all_unconnected_portals_flat(
        &self,
        nodes: &[LayoutNode],
        entrance: bool,
    ) -> Vec<(usize, usize)> {
        nodes
            .iter()
            .enumerate()
            .flat_map(|(node_index, node)| -> Vec<(usize, usize)> {
                self.unconnected_portal_indices(node, entrance)
                    .iter()
                    .map(|portal_index| (node_index, *portal_index))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>()
    }

    pub fn all_unconnected_portals(&self, nodes: &[LayoutNode], entrance: bool) -> Vec<Vec<usize>> {
        nodes
            .iter()
            .map(|node| self.unconnected_portal_indices(node, entrance))
            .collect::<Vec<_>>()
    }

    pub fn path_between_portals(&mut self, commands: &mut ChildBuilder, edge: &LayoutEdge) {
        let from_node = self.node(edge.sequence - 1, edge.from_node).clone();
        let from_portal = &from_node.room.portals[edge.from_portal];
        let to_node = self.node(edge.sequence, edge.to_node).clone();
        let to_portal = &to_node.room.portals[edge.to_portal];

        commands.spawn(LayoutPortalDebug {
            position: to_portal.transform.translation + self.rng.gen::<Vec3>(),
            direction: to_portal.direction,
        });
        commands.spawn(LayoutPortalDebug {
            position: (from_portal.transform.translation) + self.rng.gen::<Vec3>(),
            direction: from_portal.direction,
        });

        let start =
            from_portal.transform.translation - from_portal.inward() * EDGE_PATHING_RADIUS_INFLATE;
        let end =
            to_portal.transform.translation - to_portal.inward() * EDGE_PATHING_RADIUS_INFLATE;

        let real_start = from_portal.transform.translation.as_ivec3();
        let real_end = to_portal.transform.translation.as_ivec3();

        let mut points = fill_hull_with_points(&from_node, &to_node, &mut self.rng);
        points.retain(|p| {
            for seq in self.nodes.iter() {
                for node in seq {
                    let r = node.radius + EDGE_PATHING_RADIUS_INFLATE;
                    if (p.as_vec3()).distance_squared(node.position) < r * r {
                        return false;
                    }
                }
            }
            true
        });
        points.insert(0, end.as_ivec3());

        let path: Option<(Vec<IVec3>, u32)> = dijkstra(
            &start.as_ivec3(),
            |p0| -> Vec<(IVec3, u32)> {
                points
                    .iter()
                    .filter_map(|p1| {
                        if *p1 == *p0 {
                            return None;
                        }
                        if *p1 != points[0]
                            && line_segment_intersects_sphere(
                                p0.as_vec3(),
                                p1.as_vec3(),
                                to_node.position,
                                to_node.radius,
                            )
                        {
                            return None;
                        }

                        let mut cost = p0.distance_squared(*p1) as u32;

                        cost += penalize_short_hops(cost);
                        cost += penalize_steep_angles(p1, p0);

                        if *p0 == start.as_ivec3() {
                            cost += penalize_sharp_angles(&real_start, p0, p1);
                        } else if *p1 == points[0] {
                            cost += penalize_sharp_angles(p0, p1, &real_end);
                        }

                        Some((p1.clone(), cost))
                    })
                    .collect()
            },
            |p| *p == points[0],
        );

        let Some(path) = path else {
            panic!("no path");
        };

        let mut path = path.0;
        path.insert(
            0,
            (from_node.position + from_portal.transform.translation).as_ivec3(),
        );
        path.push(real_end);

        commands.spawn(LayoutPathDebug {
            color: Color::srgb(
                self.rng.gen_range(0.2..1.0),
                self.rng.gen_range(0.2..1.0),
                self.rng.gen_range(0.2..1.0),
            ),
            points,
            path,
        });
    }

    /// Arranges nodes by depenetrating them, similar to how a physics engine would.
    fn arrange_nodes(nodes: &mut [LayoutNode]) {
        // TODO this needs to push away from existing (static) rooms too
        let len = nodes.len();
        let mut done = false;
        while !done {
            done = true;

            for i in 0..len {
                for j in 0..len {
                    if i == j {
                        continue;
                    }

                    let (b_position, b_radius) = (nodes[j].position, nodes[j].radius);
                    let a = &mut nodes[i];

                    let r = a.radius + b_radius;
                    if a.position.distance_squared(b_position) < r * r {
                        let dir = (b_position - a.position).normalize();
                        a.position -= dir * 2.0;
                        done = false;
                    }
                }
            }
        }
    }
}

impl Command for StepLayoutCommand {
    fn apply(self, world: &mut World) {
        let mut system_state: SystemState<(
            Commands,
            ResMut<LayoutGraph>,
            Res<AssetCollection>,
            Query<&TerrainBrush>,
        )> = SystemState::new(world);
        let (mut commands, mut graph, assets, brushes) = system_state.get_mut(world);

        let mut curr_nodes = graph.nodes[0].clone();
        curr_nodes.shuffle(&mut graph.rng);

        let curr_positions = curr_nodes.iter().map(|node| node.position);
        let avg_position = curr_positions.sum::<Vec3>() / curr_nodes.len() as f32;
        let bias_direction = avg_position.cross(Vec3::Y).normalize();
        let start_position = avg_position + bias_direction * SEQUENCE_DISTANCE;

        // Add nodes and find portals
        let mut exit_portals = graph.all_unconnected_portals_flat(&curr_nodes, false);
        let n_next_nodes = match exit_portals.len() {
            0 => panic!("no unconnected exits"),
            1 => 1,
            _ => graph.rng.gen_range(1..=exit_portals.len()),
        };
        let mut next_nodes = (0..n_next_nodes)
            .map(|i| {
                let room = assets.random_room(&mut graph.rng).clone();
                let (min, max) = room.aabb();
                let radius = (max.distance(min) / 2.0) + NODE_ARRANGEMENT_RADIUS_INFLATE;

                LayoutNode {
                    sequence: graph.sequence,
                    index: i,
                    position: start_position + Vec3::ONE * graph.rng.gen_range(-4.0..4.0),
                    angle: graph.rng.gen_range(0.0..(PI * 2.0)),
                    radius,
                    room,
                }
            })
            .collect::<Vec<_>>();
        LayoutGraph::arrange_nodes(&mut next_nodes);

        let mut entrance_portals = graph.all_unconnected_portals(&next_nodes, true);

        // Add edges
        let mut next_edges = Vec::<LayoutEdge>::new();
        (0..next_nodes.len()).for_each(|to_node| {
            // panic happens here WARN
            let entrance_portals_len = entrance_portals[to_node].len();
            let to_portal =
                entrance_portals[to_node].remove(graph.rng.gen_range(0..entrance_portals_len));
            // WARN vvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvvv this code is evil
            // if entrance_portals[to_node].is_empty() {
            //     entrance_portals.remove(to_node);
            // }

            let (from_node, from_portal) =
                exit_portals.remove(graph.rng.gen_range(0..exit_portals.len()));

            next_edges.push(LayoutEdge {
                sequence: graph.sequence,
                from_node,
                from_portal,
                to_node,
                to_portal,
            });
        });

        // Spawn brushes
        next_nodes.iter().for_each(|node| {
            commands
                .spawn((LayoutSequence(graph.sequence), node.transform()))
                .with_children(|parent| {
                    for cavity in node.room.cavities.iter() {
                        parent.spawn(TerrainBrush::collider(
                            "",
                            graph.sequence,
                            VoxelMaterial::Invalid,
                            cavity.clone(),
                            node.transform(),
                        ));
                    }
                });
        });
        graph.nodes.insert(0, next_nodes);

        next_edges.iter().for_each(|edge| {
            commands
                .spawn(LayoutSequence(graph.sequence))
                .with_children(|parent| {
                    graph.path_between_portals(parent, edge);
                    // parent.spawn(TerrainBrush::curve(
                    //     "",
                    //     graph.sequence,
                    //     VoxelMaterial::BrownRock,
                    //     &points,
                    //     3.0,
                    // ));
                });
        });
        graph.edges.extend(next_edges);

        // TODO remove old sequences

        graph.sequence += 1;

        system_state.apply(world);
    }
}
