use avian3d::prelude::*;
use bevy::prelude::*;
use nalgebra::{Point2, Point3};

use crate::materials::LineMaterial;

use super::brush::{collider::ColliderBrushBundle, curve::CurveBrushBundle};

pub fn setup_debug_layout(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<LineMaterial>>,
) {
    if !cfg!(feature = "demo-layout") {
        return;
    }

    let rail_points = vec![
        Point3::new(200.0, -64.0, 32.0),
        Point3::new(32.0, -32.0, 32.0),
        Point3::new(-32.0, -16.0, 32.0),
        Point3::new(-32.0, 16.0, -32.0),
        Point3::new(32.0, 32.0, -32.0),
    ];
    let profile_points = vec![
        Point2::new(0.0, 0.0),
        Point2::new(-3.0, 0.0),
        Point2::new(-4.0, 1.0),
        Point2::new(-4.0, 12.0),
        Point2::new(0.0, 16.0),
        Point2::new(4.0, 12.0),
        Point2::new(4.0, 1.0),
        Point2::new(3.0, 0.0),
        Point2::new(0.0, 0.0),
    ]
    .into_iter()
    .map(|p| Point3::new(p.x / 2.0, p.y - 8.0, 0.0))
    .collect::<Vec<_>>();

    // commands.spawn(SweepBrushBundle::new(
    //     &mut meshes,
    //     &mut materials,
    //     rail_points,
    //     profile_points,
    // ));

    let points = vec![
        Point3::new(-32.0, 32.0, -32.0),
        Point3::new(32.0, 0.0, -32.0),
        Point3::new(32.0, -32.0, 32.0),
        Point3::new(-32.0, -48.0, 32.0),
    ];
    commands.spawn(CurveBrushBundle::new(
        &mut meshes,
        &mut materials,
        points,
        false,
    ));

    commands.spawn(ColliderBrushBundle::new(
        1.0,
        Collider::round_cuboid(24.0, 24.0, 24.0, 6.0),
        Transform::from_translation(Vec3::new(32.0, 0.0, 32.0)),
    ));

    commands.spawn(ColliderBrushBundle::new(
        2.0,
        Collider::cone(16.0, 36.0),
        Transform::from_translation(Vec3::new(-32.0, -48.0 - 18.0, 32.0))
            .with_rotation(Quat::from_rotation_x(std::f32::consts::PI)),
    ));
}
