use std::f32::consts::PI;

use bevy::{
    asset::RenderAssetUsages,
    prelude::Mesh,
    render::mesh::{PrimitiveTopology, VertexAttributeValues},
};
use curvo::prelude::{KnotStyle, NurbsCurve, NurbsCurve3D, Tessellation};
use nalgebra::{Const, OPoint, Point2, Point3};
use serde::{Deserialize, Serialize};
use strum::{EnumIter, EnumProperty};

use super::Environment;

// All tunnel profiles must have this number of points.
pub const TUNNEL_POINTS: usize = 12;

const TUNNEL_DEFAULT_RADIUS: f32 = 5.0;
const TUNNEL_DEFAULT_VARIANCE: f32 = 1.0;

#[derive(EnumIter, EnumProperty, Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum KnotStyleU8 {
    #[strum(props(Name = "Uniform"))]
    Uniform = 0,
    #[strum(props(Name = "Chordal"))]
    Chordal = 1,
    #[strum(props(Name = "Centripedal"))]
    Centripedal = 2,
}

impl KnotStyleU8 {
    pub fn as_curvo_knot_style(&self) -> KnotStyle {
        match self {
            KnotStyleU8::Uniform => KnotStyle::Uniform,
            KnotStyleU8::Chordal => KnotStyle::Chordal,
            KnotStyleU8::Centripedal => KnotStyle::Centripetal,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Default)]
pub struct TunnelPoint {
    pub position: Point2<f32>,
    pub variance: Point2<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Tunnel {
    pub environment: Environment,
    pub points: [TunnelPoint; TUNNEL_POINTS],
    pub knot_style: KnotStyleU8,
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
            knot_style: KnotStyleU8::Chordal,
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

        NurbsCurve3D::<f32>::try_periodic_interpolate(
            &points,
            3,
            self.knot_style.as_curvo_knot_style(),
        )
        .unwrap()
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
