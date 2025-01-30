use bevy::{
    asset::RenderAssetUsages,
    prelude::*,
    render::mesh::{PrimitiveTopology, VertexAttributeValues},
};
use nalgebra::Point3;

pub fn mesh_curve(samples: &[Point3<f32>]) -> Mesh {
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

pub fn curve_bounding_box(samples: &[Point3<f32>]) -> (Vec3, Vec3) {
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
