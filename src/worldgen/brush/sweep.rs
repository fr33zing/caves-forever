use avian3d::prelude::{Collider, FillMode, Position, Rotation, VhacdParameters};
use bevy::{
    prelude::{Mesh, *},
    render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
};
use curvo::prelude::*;
use nalgebra::{Const, DimName, Isometry, Point3, Rotation3, Translation3, Vector3};

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

impl TerrainBrush {
    pub fn sweep(
        material: VoxelMaterial,
        rail: Vec<Point3<f32>>,
        profile: Vec<Point3<f32>>,
    ) -> Self {
        let rail = NurbsCurve3D::<f32>::try_interpolate(&rail, 3).unwrap();
        let samples = rail.tessellate(Some(1e-8));
        let aabb = curve_bounding_box(&samples);
        let chunks = ChunksAABB::from_world_aabb(aabb, 1);

        let profile = NurbsCurve3D::<f32>::try_periodic(&profile, 3).unwrap();
        let sweep_mesh = sweep_zero_twist_filled::<Const<4>>(&profile, &rail, Some(4));

        let config = VhacdParameters {
            concavity: 0.01,
            alpha: 0.025,
            beta: 0.025,
            resolution: 64,
            plane_downsampling: 4,
            convex_hull_downsampling: 4,
            fill_mode: FillMode::FloodFill {
                detect_cavities: false,
            },
            convex_hull_approximation: true,
            max_convex_hulls: 1024,
        };
        let collider =
            Collider::convex_decomposition_from_mesh_with_config(&sweep_mesh, &config).unwrap();
        //let collider = Collider::trimesh_from_mesh(&sweep_mesh).unwrap();

        Self::Collider(collider, material, chunks)
    }
}

pub fn sweep_zero_twist_filled<D>(
    profile: &NurbsCurve3D<f32>,
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
        .map(|frame| {
            let translate = Translation3::from(frame.position().clone());
            let tangent = frame.tangent().clone();
            let angle = tangent.x.atan2(tangent.z);
            let rotation = Rotation3::from_axis_angle(&Vector3::y_axis(), angle);
            let transform = translate * rotation;

            if t0.is_none() {
                t0 = Some(transform.clone());
            }
            t1 = Some(transform.clone());

            profile.transformed(&transform.into())
        })
        .collect();

    let result = NurbsSurface3D::try_loft(&curves, degree_v).unwrap();

    let tessellation = result.tessellate(Some(AdaptiveTessellationOptions::default()));
    let mut sweep_mesh = mesh_tessellation(tessellation);

    let samples = profile.tessellate(Some(1e-8));
    let profile_mesh = mesh_profile_filled(&samples);

    let start_mesh = profile_mesh.clone().transformed_by(
        Transform::from_translation(t0.unwrap().translation.into())
            .with_rotation(t0.unwrap().rotation.into()),
    );
    let end_mesh = profile_mesh.transformed_by(
        Transform::from_translation(t1.unwrap().translation.into())
            .with_rotation(t1.unwrap().rotation.into()),
    );

    sweep_mesh.merge(&start_mesh);
    sweep_mesh.merge(&end_mesh);

    sweep_mesh
}

pub fn mesh_profile_filled(samples: &[Point3<f32>]) -> Mesh {
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
