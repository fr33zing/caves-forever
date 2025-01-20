use avian3d::prelude::{Collider, Position, Rotation};
use bevy::{
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
};
use curvo::prelude::*;
use nalgebra::{Const, DimName, Point3, Rotation3, Translation3, Vector3};

use super::{
    curve::{curve_bounding_box, mesh_curve},
    Sampler,
};
use crate::{
    materials::LineMaterial,
    worldgen::{
        chunk::ChunksAABB,
        voxel::{VoxelMaterial, VoxelSample},
    },
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

#[derive(Bundle)]
pub struct SweepBrushBundle {
    brush: SweepBrush,
    chunks: ChunksAABB,
    mesh: Mesh3d,
    material: MeshMaterial3d<LineMaterial>,
}

impl SweepBrushBundle {
    pub fn new(
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<LineMaterial>>,
        rail_points: Vec<Point3<f32>>,
        profile_points: Vec<Point3<f32>>,
    ) -> Self {
        let rail = NurbsCurve3D::<f32>::try_interpolate(&rail_points, 3).unwrap();
        let samples = rail.tessellate(Some(1e-8));
        let rail_mesh = mesh_curve(&samples);
        let aabb = curve_bounding_box(&samples);
        let chunks = ChunksAABB::from_world_aabb(aabb, 1);

        let profile = NurbsCurve3D::<f32>::try_periodic(&profile_points, 3).unwrap();
        let sweep = sweep_zero_twist::<f32, Const<4>>(&profile, &rail, Some(4));
        let boundary = BoundaryConstraints::default();
        let tessellation =
            sweep.constrained_tessellate(boundary, Some(AdaptiveTessellationOptions::default()));
        let sweep_mesh = mesh_tessellation(tessellation);

        let collider = Collider::convex_decomposition_from_mesh(&sweep_mesh).unwrap();

        SweepBrushBundle {
            brush: SweepBrush {
                collider,
                material: VoxelMaterial::BrownRock,
                rail,
                profile,
            },
            chunks,
            mesh: Mesh3d(meshes.add(rail_mesh)),
            material: MeshMaterial3d(materials.add(LineMaterial {
                color: Color::srgba(1.0, 1.0, 1.0, 0.1),
                opacity: 0.1,
                alpha_mode: AlphaMode::Blend,
                ..Default::default()
            })),
        }
    }
}

pub fn sweep_zero_twist<T, D>(
    profile: &NurbsCurve3D<T>,
    rail: &NurbsCurve3D<T>,
    degree_v: Option<usize>,
) -> NurbsSurface3D<T>
where
    T: FloatingPoint,
    D: DimName,
{
    let (start, end) = rail.knots_domain();
    let samples = rail.control_points().len() * 2;
    let span = (end - start) / T::from_usize(samples - 1).unwrap();

    let parameters: Vec<_> = (0..samples)
        .map(|i| start + T::from_usize(i).unwrap() * span)
        .collect();

    let frames = rail.compute_frenet_frames(&parameters);
    let curves: Vec<_> = frames
        .iter()
        .map(|frame| {
            let translate = Translation3::from(frame.position().clone());
            let tangent = frame.tangent().clone();
            let angle = tangent.x.atan2(tangent.z);
            let rotation = Rotation3::from_axis_angle(&Vector3::y_axis(), angle);
            let transform = translate * rotation;

            profile.transformed(&transform.into())
        })
        .collect();

    NurbsSurface3D::try_loft(&curves, degree_v).unwrap()
}

#[allow(unused)]
pub fn mesh_tessellation(tess: SurfaceTessellation<f32, Const<4>>) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());

    let vertices = tess.points().iter().map(|pt| (*pt).into()).collect();
    let normals = tess.normals().iter().map(|n| (*n).into()).collect();
    let uvs = tess.uvs().iter().map(|uv| (*uv).into()).collect();
    let indices = tess
        .faces()
        .iter()
        .flat_map(|f| f.iter().map(|i| *i as u32))
        .collect();

    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(vertices),
    );
    mesh.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        VertexAttributeValues::Float32x3(normals),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, VertexAttributeValues::Float32x2(uvs));
    mesh.insert_indices(Indices::U32(indices));

    mesh
}
