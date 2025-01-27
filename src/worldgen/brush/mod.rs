use anyhow::anyhow;
use avian3d::prelude::*;
use bevy::{
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use curvo::prelude::{NurbsCurve3D, Tessellation};
use nalgebra::{Const, Point3};
use sweep::{sweep_zero_twist_filled, ProfileRamp};

use super::{
    chunk::ChunksAABB,
    consts::VHACD_PARAMETERS,
    voxel::{VoxelMaterial, VoxelSample},
};

pub mod collider;
pub mod curve;
pub mod sweep;

use curve::curve_bounding_box;

pub trait Sampler {
    fn sample(&self, point: Vec3) -> VoxelSample;
}

pub struct TerrainBrushPlugin;

impl Plugin for TerrainBrushPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (process_brushes, receive_brushes));
    }
}

fn process_brushes(mut commands: Commands, requests: Query<(Entity, &TerrainBrushRequest)>) {
    let task_pool = AsyncComputeTaskPool::get();

    requests.iter().for_each(|(request_entity, request)| {
        let request = request.clone();
        let task = task_pool.spawn(async move { request.process() });

        commands.entity(request_entity).clear();
        commands.spawn(TerrainBrushTask(task));
    });
}

fn receive_brushes(mut commands: Commands, mut tasks: Query<(Entity, &mut TerrainBrushTask)>) {
    for (task_entity, mut task) in tasks.iter_mut() {
        let status = block_on(future::poll_once(&mut task.0));

        let Some(brush) = status else {
            continue;
        };

        commands.entity(task_entity).clear();
        commands.spawn(brush);
    }
}

#[derive(Component)]
struct TerrainBrushTask(Task<TerrainBrush>);

#[derive(Component, Clone)]
pub enum TerrainBrushRequest {
    Curve {
        material: VoxelMaterial,
        points: Vec<Point3<f32>>,
        radius: f32,
    },
    Sweep {
        material: VoxelMaterial,
        rail: Vec<Point3<f32>>,
        profile: ProfileRamp,
    },
}

impl TerrainBrushRequest {
    pub fn process(self) -> TerrainBrush {
        match self {
            TerrainBrushRequest::Curve {
                material,
                points,
                radius,
            } => TerrainBrush::curve(material, points, radius),
            TerrainBrushRequest::Sweep {
                material,
                rail,
                profile,
            } => TerrainBrush::sweep(material, rail.clone(), profile).unwrap_or_else(|_| {
                // TODO dynamic fallback curve radius
                TerrainBrush::curve(VoxelMaterial::Invalid, rail, 4.0)
            }),
        }
    }
}

#[derive(Component, Clone)]
pub enum TerrainBrush {
    Curve(NurbsCurve3D<f32>, f32, VoxelMaterial, ChunksAABB),
    Collider(Collider, VoxelMaterial, ChunksAABB),
}

impl TerrainBrush {
    pub fn chunks(&self) -> &ChunksAABB {
        match self {
            TerrainBrush::Curve(_, _, _, chunks_aabb) => chunks_aabb,
            TerrainBrush::Collider(_, _, chunks_aabb) => chunks_aabb,
        }
    }

    pub fn sample(&self, point: Vec3) -> VoxelSample {
        match self {
            TerrainBrush::Curve(_, _, _, _) => self.sample_curve(point),
            TerrainBrush::Collider(_, _, _) => self.sample_collider(point),
        }
    }

    //
    // Spawning
    //

    pub fn curve(material: VoxelMaterial, points: Vec<Point3<f32>>, radius: f32) -> Self {
        let curve = NurbsCurve3D::<f32>::try_interpolate(&points, 3).unwrap();
        let samples = curve.tessellate(Some(1e-8));
        let aabb = curve_bounding_box(&samples);
        let chunks = ChunksAABB::from_world_aabb(aabb, 1);

        Self::Curve(curve, radius, material, chunks)
    }

    pub fn sweep(
        material: VoxelMaterial,
        rail: Vec<Point3<f32>>,
        profile: ProfileRamp,
    ) -> anyhow::Result<Self> {
        let rail = NurbsCurve3D::<f32>::try_interpolate(&rail, 3)?;
        let samples = rail.tessellate(Some(1e-8));
        let aabb = curve_bounding_box(&samples);
        let chunks = ChunksAABB::from_world_aabb(aabb, 1);
        let sweep_mesh = sweep_zero_twist_filled::<Const<4>>(&profile, &rail, Some(4))?;
        let collider =
            Collider::convex_decomposition_from_mesh_with_config(&sweep_mesh, &VHACD_PARAMETERS)
                .ok_or_else(|| anyhow!("convex decomposition failed"))?;

        Ok(Self::Collider(collider, material, chunks))
    }

    //
    // Sampling
    //

    fn sample_curve(&self, point: Vec3) -> VoxelSample {
        let TerrainBrush::Curve(curve, radius, material, _) = self else {
            panic!("wrong sample function");
        };

        let closest: Vec3 = curve.find_closest_point(&point.into()).unwrap().into();
        let distance = point.distance(closest) - radius;

        VoxelSample {
            material: *material,
            distance,
        }
    }

    fn sample_collider(&self, point: Vec3) -> VoxelSample {
        let TerrainBrush::Collider(collider, material, _) = self else {
            panic!("wrong sample function");
        };

        let (closest, _) =
            collider.project_point(Position::default(), Rotation::default(), point, false);
        let (closest_solid, _) =
            collider.project_point(Position::default(), Rotation::default(), point, true);

        let mut distance = point.distance(closest);

        // is_inside from project_point is unreliable
        let is_inside = closest_solid.distance(point) <= 0.01;

        if is_inside {
            distance *= -1.0;
            distance = distance.min(-0.001);
        }

        VoxelSample {
            material: *material,
            distance,
        }
    }
}
