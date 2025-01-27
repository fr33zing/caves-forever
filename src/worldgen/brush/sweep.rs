use bevy::{
    prelude::{Mesh, *},
    render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
};
use curvo::prelude::*;
use earclip::earclip;
use nalgebra::{Const, DimName, Point3, Rotation3, Translation3, Vector3};

/// Facilitates interpolating between multiple sweep profiles.
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
        if self.0.len() == 2 {
            Self::lerp_profile(&mut self.0[0].1.clone(), &self.0[1].1, parameter);
        }

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
                    Self::lerp_profile(&mut profile, &w[1].1, fac);

                    return Some(profile);
                }

                None
            })
            .unwrap_or_else(|| self.0.last().unwrap().1.clone())
    }

    fn lerp_profile(a: &mut [Point3<f32>], b: &[Point3<f32>], fac: f32) {
        a.iter_mut().zip(b).for_each(|(a, b)| {
            *a = a.lerp(&b, fac);
        });
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

    let mut start_cap: Option<(
        NurbsCurve3D<f32>,
        nalgebra::Isometry<f32, nalgebra::Rotation<f32, 3>, 3>,
    )> = None;
    let mut end_cap: Option<(
        NurbsCurve3D<f32>,
        nalgebra::Isometry<f32, nalgebra::Rotation<f32, 3>, 3>,
    )> = None;

    let frames = rail.compute_frenet_frames(&parameters);
    let len = frames.len() - 1;
    let curves: Vec<_> = frames
        .into_iter()
        .enumerate()
        .map(|(i, frame)| {
            let translate = Translation3::from(*frame.position());
            let tangent = frame.tangent();
            let angle = tangent.x.atan2(tangent.z);
            let rotation = Rotation3::from_axis_angle(&Vector3::y_axis(), angle);
            let transform = translate * rotation;

            let sample = profile.sample(parameters[i]);
            let profile =
                NurbsCurve3D::try_periodic_interpolate(&sample, 3, KnotStyle::Centripetal).unwrap();

            let result = profile.transformed(&transform.into());

            if i == 0 {
                start_cap = Some((profile, transform));
            } else if i == len {
                end_cap = Some((profile, transform));
            }

            result
        })
        .collect();

    let result = NurbsSurface3D::try_loft(&curves, degree_v).unwrap();

    let tessellation = result.tessellate(Some(AdaptiveTessellationOptions::default()));
    let mut sweep_mesh = mesh_tessellation(tessellation);

    let (start_cap, end_cap) = (start_cap.unwrap(), end_cap.unwrap());
    let start_mesh = mesh_profile_filled(&start_cap.0).transformed_by(
        Transform::from_translation(start_cap.1.translation.into())
            .with_rotation(start_cap.1.rotation.into()),
    );
    let end_mesh = mesh_profile_filled(&end_cap.0).transformed_by(
        Transform::from_translation(end_cap.1.translation.into())
            .with_rotation(end_cap.1.rotation.into()),
    );

    sweep_mesh.merge(&start_mesh);
    sweep_mesh.merge(&end_mesh);

    sweep_mesh
}

pub fn mesh_profile_filled(profile: &NurbsCurve3D<f32>) -> Mesh {
    let samples = profile.tessellate(Some(1e-8));

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
    let vertices: Vec<Vec3> = samples.iter().map(|pt| (*pt).into()).collect();

    let points = vertices
        .into_iter()
        .map(|v| vec![v.x, v.y, v.z])
        .collect::<Vec<_>>();
    let points = vec![points];
    let result = earclip::<f32, u32>(&points, None, None);
    let positions = result
        .0
        .chunks(3)
        .map(|w| [w[0], w[1], w[2]])
        .collect::<Vec<[f32; 3]>>();

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_indices(Indices::U32(result.1));

    mesh
}

pub fn mesh_tessellation(tessellation: SurfaceTessellation<f32, Const<4>>) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());

    let vertices = tessellation
        .points()
        .into_iter()
        .map(|pt| (*pt).into())
        .collect();
    let indices = tessellation
        .faces()
        .into_iter()
        .flat_map(|f| f.into_iter().map(|i| *i as u32))
        .collect();

    mesh.insert_attribute(
        Mesh::ATTRIBUTE_POSITION,
        VertexAttributeValues::Float32x3(vertices),
    );
    mesh.insert_indices(Indices::U32(indices));

    mesh
}
