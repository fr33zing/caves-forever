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

// All tunnel profiles must have this number of points.
pub const TUNNEL_POINTS: usize = 12;

const TUNNEL_DEFAULT_RADIUS: f32 = 5.0;
const TUNNEL_DEFAULT_VARIANCE: f32 = 1.0;

pub struct TunnelMeshInfo {
    pub center: Vec2,
    pub size: Vec2,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
pub struct TunnelPoint {
    pub position: Point2<f32>,
    pub variance: Point2<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Tunnel {
    pub environment: Environment,
    pub rarity: Rarity,
    pub points: [TunnelPoint; TUNNEL_POINTS],
}

impl Default for Tunnel {
    fn default() -> Self {
        let mut points = [TunnelPoint::default(); TUNNEL_POINTS];
        for i in 0..TUNNEL_POINTS {
            let radians = (i as f32 / TUNNEL_POINTS as f32) * PI * 2.0;
            points[i].position = Point2::new(radians.sin(), -radians.cos()) * TUNNEL_DEFAULT_RADIUS;
            points[i].variance = Point2::new(TUNNEL_DEFAULT_VARIANCE, TUNNEL_DEFAULT_VARIANCE);
        }

        Self {
            points,
            environment: Environment::Development,
            rarity: Rarity::Uncommon,
        }
    }
}

impl Tunnel {
    pub fn to_3d(&self) -> Vec<OPoint<f32, Const<3>>> {
        self.points
            .iter()
            .map(|p| Point3::new(p.position.x, 0.0, p.position.y))
            .collect()
    }

    pub fn to_curve_3d(&self) -> NurbsCurve<f32, Const<4>> {
        let points = self.to_3d();
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
