use std::{collections::HashSet, f32::consts::PI};

use bevy::prelude::*;
use rand::Rng;

use super::{
    LayoutNode, EDGE_PATHING_RADIUS_INFLATE, HULL_DENSITY, NODE_ARRANGEMENT_RADIUS_INFLATE,
};

pub fn hull_volume(h: f32, r0: f32, r1: f32) -> f32 {
    fn hemisphere_volume(r: f32) -> f32 {
        4.0 / 3.0 * PI * (r * r * r) / 2.0
    }
    fn truncated_cone_volume(h: f32, r0: f32, r1: f32) -> f32 {
        PI / 3.0 * h * (r1 * r1 + r0 * r0 + r0 * r1)
    }

    hemisphere_volume(r0) + hemisphere_volume(r1) + truncated_cone_volume(h, r0, r1)
}

fn closest_point_on_line_segment(start: Vec3, end: Vec3, center: Vec3) -> Vec3 {
    let line = end - start;
    let len = line.length();
    let line = line.normalize();

    let v = center - start;
    let d = v.dot(line);
    let d = d.clamp(0.0, len);
    start + line * d
}

pub fn line_segment_intersects_sphere(start: Vec3, end: Vec3, center: Vec3, radius: f32) -> bool {
    let closest = closest_point_on_line_segment(start, end, center);
    closest.distance(center) < radius
}

pub fn fill_hull_with_points<R>(from: &LayoutNode, to: &LayoutNode, rng: &mut R) -> Vec<IVec3>
where
    R: Rng + ?Sized,
{
    let r0 = from.radius + NODE_ARRANGEMENT_RADIUS_INFLATE + EDGE_PATHING_RADIUS_INFLATE;
    let r1 = to.radius + NODE_ARRANGEMENT_RADIUS_INFLATE + EDGE_PATHING_RADIUS_INFLATE;
    let r2 = from.radius + EDGE_PATHING_RADIUS_INFLATE;
    let r3 = from.radius + EDGE_PATHING_RADIUS_INFLATE;
    let h = from.position.distance(to.position);

    let volume = hull_volume(h, r0, r1);
    let points = (volume * HULL_DENSITY) as usize;

    let hashset = (0..points)
        .filter_map(|_| {
            let parameter = rng.gen();
            let position = from.position.lerp(to.position, parameter);
            let direction = (rng.gen::<Vec3>() - Vec3::splat(0.5)).normalize();
            let radius = rng.gen::<f32>() * r0.lerp(r1, parameter);
            let point = position + direction * radius;

            if point.distance_squared(from.position) > r2 * r2
                && point.distance_squared(to.position) > r3 * r3
            {
                Some(point.as_ivec3())
            } else {
                None
            }
        })
        .collect::<HashSet<_>>();

    hashset.into_iter().collect()
}

pub fn penalty_curve(fault: f32) -> f32 {
    1.0 - (1.0 - fault).powi(3)
}

pub fn penalize(fault: f32, penalty: f32) -> u32 {
    (penalty_curve(fault) * penalty) as u32
}

pub fn penalize_short_hops(distance: u32) -> u32 {
    const SHORT: f32 = 16.0;
    const SHORT_SQUARED: f32 = SHORT * SHORT;
    const SHORT_PENALTY: f32 = 4096.0;

    let distance = distance as f32;
    let fault = (SHORT_SQUARED - distance) / SHORT_SQUARED;

    if distance < SHORT_SQUARED {
        penalize(fault, SHORT_PENALTY)
    } else {
        0
    }
}

pub fn penalize_sharp_angles(a: &IVec3, center: &IVec3, b: &IVec3) -> u32 {
    const SHARP_ANGLE_PENALTY: f32 = 8192.0;

    let (a, b, center) = (a.as_vec3(), b.as_vec3(), center.as_vec3());
    let dir_a = (a - center).normalize();
    let dir_b = (b - center).normalize();
    let fault = (dir_a.dot(dir_b) + 1.0) / 2.0;

    penalize(fault, SHARP_ANGLE_PENALTY)
}

pub fn penalize_steep_angles(a: &IVec3, center: &IVec3) -> u32 {
    const STEEP_ANGLE_PENALTY: f32 = 4096.0;

    let (a, center) = (a.as_vec3(), center.as_vec3());
    let b = Vec3::new(a.x, center.y, a.z);
    let dir_a = (a - center).normalize();
    let dir_b = (b - center).normalize();
    let fault = 1.0 - ((dir_a.dot(dir_b) + 1.0) / 2.0);

    penalize(fault, STEEP_ANGLE_PENALTY)
}
