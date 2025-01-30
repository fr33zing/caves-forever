use std::f32::consts::PI;

use avian3d::prelude::*;
use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
};

pub struct CableSegments {
    pub length: f32,
    pub radius: f32,
    pub faces: u16,
}

impl CableSegments {
    fn total_segments(&self, max_length: f32) -> u16 {
        (max_length / self.length).ceil().max(0.0) as u16
    }
}

#[derive(Component)]
pub struct CableStart;

#[derive(Component)]
pub struct CableEnd;

#[derive(Component)]
pub struct CableSegment;

#[derive(Component)]
pub struct CableSkinnedMeshJoint(pub Entity);

pub struct CablePlugin;

impl Plugin for CablePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sync_joints);
    }
}

fn sync_joints(
    segments: Query<&GlobalTransform, With<CableSegment>>,
    mut joints: Query<(&CableSkinnedMeshJoint, &mut Transform), Without<CableSegment>>,
) {
    for (joint, mut transform) in joints.iter_mut() {
        if let Ok(segment) = segments.get(joint.0) {
            let (a, b, _) = segment.rotation().to_euler(EulerRot::XZY);
            let rotation_untwisted = Quat::from_euler(EulerRot::XZY, a, b, 0.0);

            transform.rotation = rotation_untwisted;
            transform.translation = segment.translation();
        }
    }
}

pub fn generate_colliders(max_length: f32, segments: &CableSegments) -> Vec<(Collider, f32)> {
    let &CableSegments { length, radius, .. } = segments;

    let mut colliders = Vec::<(Collider, f32)>::new();
    let skin = 0.01;

    for i in 0..segments.total_segments(max_length) {
        colliders.push((
            Collider::capsule(radius, length - radius * 2.0 - skin),
            length * i as f32 + skin / 2.0,
        ));
    }

    colliders
}

pub fn generate_mesh(max_length: f32, segments: &CableSegments) -> (Mesh, Vec<Mat4>) {
    let &CableSegments {
        length,
        radius,
        faces,
    } = segments;

    let mut positions = Vec::<[f32; 3]>::new();
    let mut indices = Vec::<u16>::new();
    let mut joint_indices = Vec::<[u16; 4]>::new();
    let mut joint_weights = Vec::<[f32; 4]>::new();
    let mut inverse_bindposes = Vec::<Mat4>::new();

    let mut add_ring = |ring: u16| {
        let y = length * ring as f32;
        // TODO figure out why rope is misaligned without this offset
        //let y = y - length / 2.0;

        inverse_bindposes.push(Mat4::from_translation(Vec3::new(
            0.0,
            -y - length / 2.0,
            0.0,
        )));
        for i in 0..faces {
            let theta = i as f32 / faces as f32 * 2.0 * PI;
            let x = theta.cos() * radius;
            let z = theta.sin() * radius;

            positions.push([x, y, z]);
            joint_indices.push([ring, 0, 0, 0]);
            joint_weights.push([1.0, 0.0, 0.0, 0.0]);
            // TODO add UVs here
        }
    };

    let mut add_segment = |segment: u16| {
        // Positions
        if segment == 0 {
            add_ring(0);
        }
        add_ring(segment + 1);

        // Indices
        for i in 0..faces {
            let bottom = |index: u16| -> u16 { (index + i) % faces + faces * segment };
            let top = |index: u16| -> u16 { bottom(index) + faces };

            indices.extend(vec![
                // 0
                bottom(1),
                bottom(0),
                top(0),
                // 1
                top(0),
                top(1),
                bottom(1),
            ]);

            // TODO add normals here (?)
        }
    };

    // Generate mesh
    for i in 0..segments.total_segments(max_length) {
        add_segment(i);
    }

    let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::all())
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_indices(Indices::U16(indices))
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_JOINT_INDEX,
            VertexAttributeValues::Uint16x4(joint_indices),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, joint_weights)
        .with_computed_normals();

    (mesh, inverse_bindposes)
}
