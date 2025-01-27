use avian3d::prelude::{Collider, FillMode, Position, Rotation, VhacdParameters};
use bevy::{
    prelude::{Mesh, *},
    render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
};
use curvo::prelude::*;
use nalgebra::{Const, DimName, Point3, Rotation3, Translation3, Vector3};

use super::{curve::curve_bounding_box, Sampler, TerrainBrush};
use crate::worldgen::{
    chunk::ChunksAABB,
    voxel::{VoxelMaterial, VoxelSample},
};

#[derive(Component, Clone)]
pub struct SweepBrush {
    pub collider: Collider,
    pub material: VoxelMaterial,
    pub rail: NurbsCurve3D<f32>,
    pub profile: NurbsCurve3D<f32>,
}

impl Sampler for SweepBrush {
    fn sample(&self, point: Vec3) -> VoxelSample {
        let (closest, _) =
            self.collider
                .project_point(Position::default(), Rotation::default(), point, false);
        let (closest_solid, _) =
            self.collider
                .project_point(Position::default(), Rotation::default(), point, true);

        let mut distance = point.distance(closest);

        // is_inside from project_point is unreliable
        let is_inside = closest_solid.distance(point) <= 0.01;

        if is_inside {
            distance *= -1.0;
            distance = distance.min(-0.001);
        }

        let material = self.material;

        VoxelSample { material, distance }
    }
}

#[derive(Clone)]
pub struct ProfileRamp(Vec<(f32, Vec<Point3<f32>>)>);

impl ProfileRamp {
    pub fn start(profile: Vec<Point3<f32>>) -> Self {
        Self(vec![(0.0, profile)])
    }

    pub fn end(mut self, profile: Vec<Point3<f32>>) -> Self {
        self.0.push((1.0, profile));
        self
    }

    pub fn point(mut self, parameter: f32, profile: Vec<Point3<f32>>) -> Self {
        let mut i: usize = 0;
        while i < self.0.len() {
            if self.0[i].0 > parameter {
                break;
            }

            i += 1;
        }
        self.0.insert(i, (parameter, profile));

        self
    }

    pub fn sample(&self, parameter: f32) -> Vec<Point3<f32>> {
        self.0
            .windows(2)
            .find_map(|w| {
                if w[0].0 == parameter {
                    return Some(w[0].1.clone());
                }

                if w[1].0 > parameter {
                    let diff = w[1].0 - w[0].0;
                    let fac = (parameter - w[0].0) / diff;
                    let mut profile = w[0].1.clone();
                    profile.iter_mut().enumerate().for_each(|(i, p)| {
                        *p = p.lerp(&w[1].1[i], fac);
                    });

                    return Some(profile);
                }

                None
            })
            .unwrap_or_else(|| self.0.last().unwrap().1.clone())
    }
}

impl TerrainBrush {
    pub fn sweep(material: VoxelMaterial, rail: Vec<Point3<f32>>, profile: ProfileRamp) -> Self {
        let rail = NurbsCurve3D::<f32>::try_interpolate(&rail, 3).unwrap();
        let samples = rail.tessellate(Some(1e-8));
        let aabb = curve_bounding_box(&samples);
        let chunks = ChunksAABB::from_world_aabb(aabb, 1);

        let sweep_mesh = sweep_zero_twist_filled::<Const<4>>(&profile, &rail, Some(4));

        let config = VhacdParameters {
            alpha: 0.025,
            beta: 0.025,
            ..default()
        };
        let collider =
            Collider::convex_decomposition_from_mesh_with_config(&sweep_mesh, &config).unwrap();

        Self::Collider(collider, material, chunks)
    }
}

pub fn sweep_zero_twist_filled<D>(
    profile: &ProfileRamp,
    rail: &NurbsCurve3D<f32>,
    degree_v: Option<usize>,
) -> Mesh
where
    D: DimName,
{
    let (start, end) = rail.knots_domain();
    let samples = rail.control_points().len() * 2;
    let span = (end - start) / (samples - 1) as f32;

    let parameters: Vec<_> = (0..samples).map(|i| start + i as f32 * span).collect();

    let mut t0: Option<nalgebra::Isometry<f32, nalgebra::Rotation<f32, 3>, 3>> = None;
    let mut t1: Option<nalgebra::Isometry<f32, nalgebra::Rotation<f32, 3>, 3>> = None;

    let frames = rail.compute_frenet_frames(&parameters);
    let curves: Vec<_> = frames
        .iter()
        .enumerate()
        .map(|(i, frame)| {
            let translate = Translation3::from(frame.position().clone());
            let tangent = frame.tangent().clone();
            let angle = tangent.x.atan2(tangent.z);
            let rotation = Rotation3::from_axis_angle(&Vector3::y_axis(), angle);
            let transform = translate * rotation;

            if t0.is_none() {
                t0 = Some(transform.clone());
            }
            t1 = Some(transform.clone());

            let sample = profile.sample(parameters[i]);
            let profile =
                NurbsCurve3D::try_periodic_interpolate(&sample, 3, KnotStyle::Centripetal).unwrap();
            profile.transformed(&transform.into())
        })
        .collect();

    let result = NurbsSurface3D::try_loft(&curves, degree_v).unwrap();

    let tessellation = result.tessellate(Some(AdaptiveTessellationOptions::default()));
    let mut sweep_mesh = mesh_tessellation(tessellation);

    let start_mesh = mesh_profile_filled(&profile.sample(0.0)).transformed_by(
        Transform::from_translation(t0.unwrap().translation.into())
            .with_rotation(t0.unwrap().rotation.into()),
    );
    let end_mesh = mesh_profile_filled(&profile.sample(1.0)).transformed_by(
        Transform::from_translation(t1.unwrap().translation.into())
            .with_rotation(t1.unwrap().rotation.into()),
    );

    sweep_mesh.merge(&start_mesh);
    sweep_mesh.merge(&end_mesh);

    sweep_mesh
}

pub fn mesh_profile_filled(profile: &[Point3<f32>]) -> Mesh {
    let profile =
        NurbsCurve3D::try_periodic_interpolate(&profile, 3, KnotStyle::Centripetal).unwrap();
    let samples = profile.tessellate(Some(1e-8));

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    let vertices: Vec<Vec3> = samples.iter().map(|pt| (*pt).into()).collect();

    let points = vertices
        .iter()
        .map(|v| vec![v.x, v.y, v.z])
        .collect::<Vec<_>>();
    let points = vec![points];
    let result = earclip::earclip::<f32, u32>(&points, None, None);
    let positions = result
        .0
        .chunks(3)
        .map(|w| [w[0], w[1], w[2]])
        .collect::<Vec<[f32; 3]>>();

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(Indices::U32(result.1));

    mesh
}

#[allow(unused)]
pub fn mesh_tessellation(tess: SurfaceTessellation<f32, Const<4>>) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());

    let mut vertices = tess.points().iter().map(|pt| (*pt).into()).collect();
    let mut indices = tess
        .faces()
        .iter()
        .flat_map(|f| f.iter().map(|i| *i as u32))
        .collect();

    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(vertices),
    );
    mesh.insert_indices(Indices::U32(indices));

    mesh
}
