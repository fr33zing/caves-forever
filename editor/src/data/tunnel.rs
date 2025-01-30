use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    math::Vec2,
    prelude::Mesh,
    render::mesh::{PrimitiveTopology, VertexAttributeValues},
};
use curvo::prelude::{KnotStyle, NurbsCurve, NurbsCurve3D, Tessellation};
use nalgebra::{Const, OPoint, Point2, Point3};
use serde::{Deserialize, Serialize};

use super::{Environment, Rarity};
use lib::worldgen::asset::TUNNEL_POINTS;

const TUNNEL_DEFAULT_RADIUS: f32 = 5.0;

pub struct TunnelMeshInfo {
    pub center: Vec2,
    pub size: Vec2,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Tunnel {
    pub environment: Environment,
    pub rarity: Rarity,
    pub points: [Point2<f32>; TUNNEL_POINTS],
}

impl Default for Tunnel {
    fn default() -> Self {
        let mut points = [Point2::<f32>::default(); TUNNEL_POINTS];
        for i in 0..TUNNEL_POINTS {
            let radians = (i as f32 / TUNNEL_POINTS as f32) * PI * 2.0;
            points[i] = Point2::new(radians.sin(), -radians.cos()) * TUNNEL_DEFAULT_RADIUS;
        }

        Self {
            points,
            environment: Environment::Development,
            rarity: Rarity::Uncommon,
        }
    }
}

impl Tunnel {
    pub fn build(&self) -> lib::worldgen::asset::Tunnel {
        lib::worldgen::asset::Tunnel {
            weight: self.rarity.weight(),
            points: self.points,
        }
    }

    pub fn to_3d_xz(&self) -> Vec<OPoint<f32, Const<3>>> {
        self.points
            .iter()
            .map(|p| Point3::new(p.x, 0.0, p.y))
            .collect()
    }

    pub fn to_3d_xy_scaled(&self, scale: Vec2) -> Vec<OPoint<f32, Const<3>>> {
        self.points
            .iter()
            .map(|p| Point3::new(p.x * scale.x, p.y * scale.y, 0.0))
            .collect()
    }

    pub fn to_curve_3d(&self) -> NurbsCurve<f32, Const<4>> {
        let points = self.to_3d_xz();
        NurbsCurve3D::<f32>::try_periodic_interpolate(&points, 3, KnotStyle::Centripetal).unwrap()
    }

    pub fn to_mesh(&self) -> Mesh {
        let curve = self.to_curve_3d();
        let samples = curve.tessellate(Some(1e-8));
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

    pub fn center(&mut self) {
        let info = TunnelMeshInfo::from_mesh(&self.to_mesh());
        for point in self.points.iter_mut() {
            point.x -= info.center.x;
            point.y -= info.center.y;
        }
    }
}

impl TunnelMeshInfo {
    pub const ZERO: Self = Self {
        size: Vec2::ZERO,
        center: Vec2::ZERO,
    };

    pub fn from_mesh(mesh: &Mesh) -> Self {
        let mut min = Vec2::INFINITY;
        let mut max = Vec2::NEG_INFINITY;

        let Some(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
            return Self::ZERO;
        };
        let Some(positions) = positions.as_float3() else {
            return Self::ZERO;
        };

        positions.iter().for_each(|p| {
            min.x = min.x.min(p[0]);
            min.y = min.y.min(p[2]);
            max.x = max.x.max(p[0]);
            max.y = max.y.max(p[2]);
        });

        let size = max - min;
        let center = min + size / 2.0;

        Self { size, center }
    }
}
