use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{PrimitiveTopology, VertexAttributeValues},
};
use curvo::prelude::*;
use nalgebra::Point3;

use super::Sampler;
use crate::{
    materials::LineMaterial,
    worldgen::{
        chunk::ChunksAABB,
        voxel::{VoxelMaterial, VoxelSample},
    },
};

#[derive(Component)]
pub struct CurveBrush {
    pub material: VoxelMaterial,
    pub curve: NurbsCurve3D<f32>,
    pub flat_bottom: bool,
}

impl Sampler for CurveBrush {
    fn sample(&self, point: Vec3) -> VoxelSample {
        let point = Point3::<f32>::new(point.x, point.y, point.z);

        let closest = self.curve.find_closest_point(&point).unwrap();
        let distance = ((closest.x - point.x).powf(2.0)
            + (closest.y - point.y).powf(2.0)
            + (closest.z - point.z).powf(2.0))
        .sqrt();

        let width = 16.0;

        VoxelSample {
            material: self.material,
            distance: distance - width,
        }
    }
}

#[derive(Bundle)]
pub struct CurveBrushBundle {
    brush: CurveBrush,
    chunks: ChunksAABB,
    mesh: Mesh3d,
    material: MeshMaterial3d<LineMaterial>,
}

impl CurveBrushBundle {
    pub fn new(
        meshes: &mut ResMut<Assets<Mesh>>,
        materials: &mut ResMut<Assets<LineMaterial>>,
        points: Vec<Point3<f32>>,
        flat_bottom: bool,
    ) -> Self {
        let curve = NurbsCurve3D::<f32>::try_interpolate(&points, 3).unwrap();
        let samples = curve.tessellate(Some(1e-8));
        let mesh = mesh_curve(&samples);
        let aabb = curve_bounding_box(&samples);
        let chunks = ChunksAABB::from_world_aabb(aabb, 1);

        CurveBrushBundle {
            brush: CurveBrush {
                material: VoxelMaterial::from_repr(0).unwrap(),
                curve,
                flat_bottom,
            },
            chunks,
            mesh: Mesh3d(meshes.add(mesh)),
            material: MeshMaterial3d(materials.add(LineMaterial {
                color: Color::srgba(1.0, 1.0, 1.0, 0.1),
                opacity: 0.1,
                alpha_mode: AlphaMode::Blend,
                ..Default::default()
            })),
        }
    }
}

fn mesh_curve(samples: &[Point3<f32>]) -> Mesh {
    let vertices = samples
        .iter()
        .map(|p| p.cast::<f32>())
        .map(|p| [p.x, p.y, p.z])
        .collect();

    Mesh::new(PrimitiveTopology::LineStrip, RenderAssetUsages::all()).with_inserted_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(vertices),
    )
}

fn curve_bounding_box(samples: &[Point3<f32>]) -> (Vec3, Vec3) {
    let mut min = Vec3::ZERO;
    let mut max = Vec3::ZERO;

    for sample in samples {
        min.x = f32::min(min.x, sample.x);
        min.y = f32::min(min.y, sample.y);
        min.z = f32::min(min.z, sample.z);
        max.x = f32::max(max.x, sample.x);
        max.y = f32::max(max.y, sample.y);
        max.z = f32::max(max.z, sample.z);
    }

    (min, max)
}
