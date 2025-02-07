use avian3d::prelude::{Collider, Position, Rotation};
use bevy::prelude::*;
use curvo::prelude::{NurbsCurve3D, Tessellation};
use nalgebra::Point3;
use rand::Rng;

use crate::{
    materials::LineMaterial,
    worldgen::{
        brush::{curve::mesh_curve, TerrainBrush},
        voxel::VoxelMaterial,
    },
};

use super::{
    consts::{ROOM_SHYNESS, TUNNEL_SHYNESS},
    room::{Portal, Room},
    utility::{find_path_between_portals, navigable_pointcloud, Arrangement},
    LayoutState,
};

#[derive(Component)]
pub struct PendingPortalConnection {
    pub sequence: usize,
    pub from_portal: Entity,
    pub to_portal: Entity,
}

#[derive(Component)]
pub struct PortalConnection {
    pub sequence: usize,
    // DEBUG
    pub color: Color,
    pub path: Vec<Vec3>,
}

pub fn connect_portals(
    mut commands: Commands,
    mut state: ResMut<LayoutState>,
    mut portals: Query<(&mut Portal, &GlobalTransform, &Parent)>,
    rooms: Query<(&Room, &GlobalTransform)>,
    arrangements: Query<&Arrangement>,
    pending: Query<(Entity, &PendingPortalConnection)>,
    //TEMP
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
) {
    if pending.is_empty() {
        return;
    }

    let mut arrangements = arrangements.iter().cloned().collect::<Vec<_>>();

    pending.iter().for_each(|(pending_entity, pending)| {
        let [from_portal, to_portal] = portals
            .get_many_mut([pending.from_portal, pending.to_portal])
            .expect("nonexistent portal(s)");
        let (mut from_portal, from_portal_transform, from_portal_parent) = from_portal;
        let (mut to_portal, to_portal_transform, to_portal_parent) = to_portal;
        let [from_room, to_room] = rooms
            .get_many([from_portal_parent.get(), to_portal_parent.get()])
            .expect("nonexistent room(s)");
        let (from_room, from_room_transform) = from_room;
        let (to_room, to_room_transform) = to_room;

        let path = 'pathfinding: {
            let max_attempts = 8;
            for attempt in 1..=max_attempts {
                let navigation_cloud = navigable_pointcloud(
                    (from_room_transform.translation(), from_room.radius),
                    (to_room_transform.translation(), to_room.radius),
                    attempt * 2,
                    &mut state.rng,
                );
                let real_start = from_portal_transform.translation();
                let real_end = to_portal_transform.translation();
                let start_offset = from_room.radius + ROOM_SHYNESS;
                let end_offset = to_room.radius + ROOM_SHYNESS;
                let pathfinding_start = (real_start
                    - from_portal.inward(from_portal_transform) * start_offset)
                    .as_ivec3();
                let pathfinding_end =
                    (real_end - to_portal.inward(to_portal_transform) * end_offset).as_ivec3();

                let path = find_path_between_portals(
                    real_start,
                    real_end,
                    pathfinding_start,
                    pathfinding_end,
                    navigation_cloud,
                    &arrangements,
                );

                if let Some(path) = path {
                    break 'pathfinding path;
                }
            }
            panic!("no viable path found after {max_attempts} attempts");
        };

        let arrangement_colliders = path
            .windows(2)
            .map(|w| {
                (
                    Position::default(),
                    Rotation::default(),
                    Collider::capsule_endpoints(TUNNEL_SHYNESS, w[0], w[1]),
                )
            })
            .collect();

        let color = Color::hsl(state.rng.gen_range(0.0..360.0), 1.0, 0.5);
        commands
            .spawn(PortalConnection {
                sequence: pending.sequence,
                color,
                path: path.clone(),
            })
            .with_children(|parent| {
                //TEMP
                let points = &mut path
                    .iter()
                    .map(|point| (*point).into())
                    .collect::<Vec<Point3<f32>>>();
                let Ok(curve) = NurbsCurve3D::<f32>::try_interpolate(&points, 3) else {
                    return;
                };
                let samples = curve.tessellate(Some(1e-8));
                let mesh = mesh_curve(&samples);
                parent.spawn((
                    Mesh3d(meshes.add(mesh)),
                    MeshMaterial3d(materials.add(LineMaterial {
                        color: color.with_alpha(0.05),
                        opacity: 0.05,
                        alpha_mode: AlphaMode::Blend,
                    })),
                ));
                parent.spawn(TerrainBrush::curve(
                    "",
                    state.sequence,
                    VoxelMaterial::BrownRock,
                    &points,
                    6.0,
                ));

                let arrangement = Arrangement {
                    collider: Collider::compound(arrangement_colliders),
                    position: default(),
                    rotation: default(),
                };
                arrangements.push(arrangement.clone());
                parent.spawn(arrangement);
            });

        from_portal.connected = true;
        to_portal.connected = true;

        commands.entity(pending_entity).despawn();
    });
}
