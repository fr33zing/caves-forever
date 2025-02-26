use std::{collections::HashSet, f32::consts::PI};

use avian3d::prelude::*;
use bevy::prelude::*;
use contact_query::contact;
use pathfinding::prelude::dijkstra;
use rand::Rng;

use crate::worldgen::layout::consts::{DEPENETRATION_DISTANCE, SHORT_HOP};

use super::consts::{HULL_DENSITY, ROOM_SHYNESS, TUNNEL_SHYNESS};

#[derive(Component, Clone)]
pub struct Arrangement {
    pub spherical: bool,
    pub collider: Collider,
    pub position: Position,
    pub rotation: Rotation,
}
impl Arrangement {
    pub fn transform(&self) -> Transform {
        Transform::from_rotation(*self.rotation).with_translation(*self.position)
    }
}

pub fn arrange_by_depenetration(
    dynamic_colliders: &mut [Arrangement],
    static_colliders: Vec<Arrangement>,
) {
    fn depenetrate(
        static_collider: &Arrangement,
        dynamic_collider: &mut Arrangement,
        desperation: f32,
    ) -> bool {
        let Some(contact) = contact(
            &dynamic_collider.collider,
            dynamic_collider.position,
            dynamic_collider.rotation,
            &static_collider.collider,
            static_collider.position,
            static_collider.rotation,
            TUNNEL_SHYNESS * 2.0, // Extra space to make room for tunnels
        )
        .expect("unsupported collider shape") else {
            return true;
        };

        let direction = if dynamic_collider.spherical && static_collider.spherical {
            (static_collider.position.0 - dynamic_collider.position.0).normalize()
        } else {
            contact.normal1
        };

        // Prefer horizontal depenetration to minimize steep paths
        let y_scale = 0.01;
        let xz_scale = 1.0 + (1.0 - y_scale) / 2.0;
        let scale = Vec3::new(xz_scale, y_scale, xz_scale);

        *dynamic_collider.position -= direction * scale * DEPENETRATION_DISTANCE * desperation;

        false
    }

    let len = dynamic_colliders.len();
    let mut done = false;
    let mut desperation = 1.0;
    let acceleration = 1.01;

    while !done {
        done = true;

        for i in 0..len {
            for j in 0..len {
                if i == j {
                    continue;
                }
                done &= depenetrate(
                    &dynamic_colliders[j].clone(),
                    &mut dynamic_colliders[i],
                    desperation,
                );
                desperation *= acceleration;
            }

            for static_collider in static_colliders.iter() {
                done &= depenetrate(static_collider, &mut dynamic_colliders[i], desperation);
                desperation *= acceleration;
            }
        }
    }
}

//
// Pathfinding
//

pub fn navigable_hull_volume(h: f32, r0: f32, r1: f32) -> f32 {
    fn hemisphere_volume(r: f32) -> f32 {
        4.0 / 3.0 * PI * (r * r * r) / 2.0
    }
    fn truncated_cone_volume(h: f32, r0: f32, r1: f32) -> f32 {
        PI / 3.0 * h * (r1 * r1 + r0 * r0 + r0 * r1)
    }

    hemisphere_volume(r0) + hemisphere_volume(r1) + truncated_cone_volume(h, r0, r1)
}

pub fn navigable_pointcloud<R>(
    from_sphere: (Vec3, f32),
    to_sphere: (Vec3, f32),
    shells: u8,
    rng: &mut R,
) -> Vec<IVec3>
where
    R: Rng + ?Sized,
{
    let navigable_shell_thickness = TUNNEL_SHYNESS * shells.max(1) as f32;
    let exclusion_radius_0 = from_sphere.1 + ROOM_SHYNESS;
    let exclusion_radius_1 = from_sphere.1 + ROOM_SHYNESS;
    let navigable_radius_0 = exclusion_radius_0 + navigable_shell_thickness;
    let navigable_radius_1 = exclusion_radius_1 + navigable_shell_thickness;
    let distance = from_sphere.0.distance(to_sphere.0);

    let volume = navigable_hull_volume(distance, navigable_radius_0, navigable_radius_1);
    let points = (volume * HULL_DENSITY) as usize;

    let hashset = (0..points)
        .map(|_| {
            let parameter = rng.gen();
            let position = from_sphere.0.lerp(to_sphere.0, parameter);
            let direction = (rng.gen::<Vec3>() - Vec3::splat(0.5)).normalize();
            let radius = rng.gen::<f32>() * navigable_radius_0.lerp(navigable_radius_1, parameter);
            let mut point = position + direction * radius;

            if point.distance_squared(from_sphere.0) < exclusion_radius_0 * exclusion_radius_0 {
                let direction_from_center = (point - from_sphere.0).normalize();
                point = from_sphere.0 + direction_from_center * navigable_radius_0;
            } else if point.distance_squared(to_sphere.0) < exclusion_radius_1 * exclusion_radius_1
            {
                let direction_from_center = (point - to_sphere.0).normalize();
                point = to_sphere.0 + direction_from_center * navigable_radius_1;
            }

            point.as_ivec3()
        })
        .collect::<HashSet<_>>();

    hashset.into_iter().collect()
}

pub fn find_path_between_portals(
    fail_on_intersection: bool,
    real_start: Vec3,
    real_end: Vec3,
    pathfinding_start: IVec3,
    pathfinding_end: IVec3,
    mut points: Vec<IVec3>,
    arrangements: &[Arrangement],
) -> Option<Vec<Vec3>> {
    points.push(pathfinding_end);

    let path = dijkstra(
        &pathfinding_start,
        |p0| -> Vec<(IVec3, u32)> {
            points
                .iter()
                .filter_map(|p1| {
                    let (p0f, p1f) = (p0.as_vec3(), p1.as_vec3());

                    let mut cost = p0.distance_squared(*p1) as u32;
                    cost += penalize_short_hops(cost);
                    cost += penalize_steep_angles(&p1f, &p0f);

                    if !is_line_navigable(&p0f, &p1f, arrangements) {
                        if fail_on_intersection {
                            return None;
                        } else {
                            cost += 80960;
                        }
                    }

                    if *p0 == pathfinding_start {
                        cost += penalize_sharp_angles(&real_start, &p0f, &p1f);
                    } else if *p1 == pathfinding_end {
                        cost += penalize_sharp_angles(&p0f, &p1f, &real_end);
                    }

                    Some((p1.clone(), cost))
                })
                .collect()
        },
        |p| *p == pathfinding_end,
    );

    let Some(path) = path else {
        return None;
    };

    let mut path = path
        .0
        .into_iter()
        .map(|point| point.as_vec3())
        .collect::<Vec<_>>();

    // Make sure the ends are straight when we interpolate
    path.insert(0, (real_start + path[0]) / 2.0);
    path.insert(0, real_start);
    path.push((real_end + path.last().unwrap()) / 2.0);
    path.push(real_end);

    Some(path)
}

fn is_line_navigable(start: &Vec3, end: &Vec3, arrangements: &[Arrangement]) -> bool {
    !arrangements.iter().any(|arrangement| {
        arrangement.collider.intersects_ray(
            arrangement.position,
            arrangement.rotation,
            *start,
            (end - start).normalize(),
            start.distance(*end),
        )
    })
}

fn is_line_navigable2(start: &Vec3, end: &Vec3, arrangements: &[Arrangement]) -> bool {
    let c = Collider::capsule_endpoints(TUNNEL_SHYNESS, *start, *end);
    !arrangements.iter().any(|arrangement| {
        contact(
            &c,
            Position::default(),
            Rotation::default(),
            &arrangement.collider,
            arrangement.position,
            arrangement.rotation,
            TUNNEL_SHYNESS * 2.0,
        )
        .is_ok_and(|x| x.is_some())
    })
}

pub fn penalty_curve(fault: f32) -> f32 {
    1.0 - (1.0 - fault).powi(3)
}

pub fn penalize(fault: f32, penalty: f32) -> u32 {
    (penalty_curve(fault) * penalty) as u32
}

pub fn penalize_short_hops(distance: u32) -> u32 {
    const SHORT_SQUARED: f32 = SHORT_HOP * SHORT_HOP;
    const SHORT_PENALTY: f32 = 4096.0;

    let distance = distance as f32;
    let fault = (SHORT_SQUARED - distance) / SHORT_SQUARED;

    if distance < SHORT_SQUARED {
        penalize(fault, SHORT_PENALTY)
    } else {
        0
    }
}

pub fn penalize_sharp_angles(a: &Vec3, center: &Vec3, b: &Vec3) -> u32 {
    const SHARP_ANGLE_PENALTY: f32 = 8192.0;

    let dir_a = (a - center).normalize();
    let dir_b = (b - center).normalize();
    let fault = (dir_a.dot(dir_b) + 1.0) / 2.0;

    penalize(fault, SHARP_ANGLE_PENALTY)
}

pub fn penalize_steep_angles(a: &Vec3, center: &Vec3) -> u32 {
    const STEEP_ANGLE_PENALTY: f32 = 4096.0;

    let b = Vec3::new(a.x, center.y, a.z);
    let dir_a = (a - center).normalize();
    let dir_b = (b - center).normalize();
    let fault = 1.0 - ((dir_a.dot(dir_b) + 1.0) / 2.0);

    penalize(fault, STEEP_ANGLE_PENALTY)
}
