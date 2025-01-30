use anyhow::anyhow;
use avian3d::prelude::*;
use bevy::{
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use curvo::prelude::{NurbsCurve3D, Tessellation};
use nalgebra::{Const, Point3};
use serde::Serialize;

use super::{
    chunk::ChunksAABB,
    consts::{VHACD_PARAMETERS, VOXEL_REAL_SIZE},
    voxel::{VoxelMaterial, VoxelSample},
};

pub mod curve;
pub mod sweep;

use curve::curve_bounding_box;
use sweep::{sweep_zero_twist_filled, ProfileRamp};

#[derive(Component)]
struct TerrainBrushTask(Task<TerrainBrush>);

#[derive(Component, Clone)]
pub enum TerrainBrushRequest {
    Curve {
        uuid: String,
        material: VoxelMaterial,
        points: Vec<Point3<f32>>,
        radius: f32,
    },
    Sweep {
        uuid: String,
        material: VoxelMaterial,
        rail: Vec<Point3<f32>>,
        profile: ProfileRamp,
    },
    Mesh {
        uuid: String,
        material: VoxelMaterial,
        mesh: Mesh,
        transform: Transform,
    },
}

#[derive(Component, Clone)]
pub enum TerrainBrush {
    Curve {
        uuid: String,
        curve: NurbsCurve3D<f32>,
        radius: f32,
        material: VoxelMaterial,
        chunks: ChunksAABB,
    },
    Collider {
        uuid: String,
        collider: Collider,
        material: VoxelMaterial,
        chunks: ChunksAABB,
    },
}

impl TerrainBrushRequest {
    pub fn process(self) -> TerrainBrush {
        match self {
            TerrainBrushRequest::Curve {
                uuid,
                material,
                points,
                radius,
            } => TerrainBrush::curve(&uuid, material, &points, radius),
            TerrainBrushRequest::Sweep {
                uuid,
                material,
                rail,
                profile,
            } => TerrainBrush::sweep(&uuid, material, &rail, &profile).unwrap_or_else(|_| {
                // TODO dynamic fallback curve radius
                TerrainBrush::curve(&uuid, VoxelMaterial::Invalid, &rail, 4.0)
            }),
            TerrainBrushRequest::Mesh {
                uuid,
                material,
                mesh,
                transform,
            } => TerrainBrush::mesh(&uuid, material, &mesh, Some(transform)).unwrap_or_else(|_| {
                // TODO dynamic fallback sphere radius
                TerrainBrush::collider(&uuid, VoxelMaterial::Invalid, Collider::sphere(4.0))
            }),
        }
    }
}

impl TerrainBrush {
    pub fn uuid(&self) -> &str {
        match self {
            TerrainBrush::Curve { uuid, .. } => uuid,
            TerrainBrush::Collider { uuid, .. } => uuid,
        }
    }

    pub fn chunks(&self) -> &ChunksAABB {
        match self {
            TerrainBrush::Curve { chunks, .. } => chunks,
            TerrainBrush::Collider { chunks, .. } => chunks,
        }
    }

    pub fn sample(&self, point: Vec3) -> VoxelSample {
        match self {
            TerrainBrush::Curve { .. } => self.sample_curve(point),
            TerrainBrush::Collider { .. } => self.sample_collider(point),
        }
    }

    //
    // Spawning
    //

    pub fn curve(uuid: &str, material: VoxelMaterial, points: &[Point3<f32>], radius: f32) -> Self {
        let curve = NurbsCurve3D::<f32>::try_interpolate(points, 3).unwrap();
        let samples = curve.tessellate(Some(1e-8));
        let aabb = curve_bounding_box(&samples);
        let chunks = ChunksAABB::from_world_aabb(aabb, 1);

        Self::Curve {
            uuid: uuid.to_owned(),
            curve,
            radius,
            material,
            chunks,
        }
    }

    pub fn sweep(
        uuid: &str,
        material: VoxelMaterial,
        rail: &[Point3<f32>],
        profile: &ProfileRamp,
    ) -> anyhow::Result<Self> {
        let rail = NurbsCurve3D::<f32>::try_interpolate(rail, 3)?;
        let mesh = sweep_zero_twist_filled::<Const<4>>(profile, &rail, Some(4))?;

        Self::mesh(uuid, material, &mesh, None)
    }

    pub fn mesh(
        uuid: &str,
        material: VoxelMaterial,
        mesh: &Mesh,
        transform: Option<Transform>,
    ) -> anyhow::Result<Self> {
        let mesh = if let Some(transform) = transform {
            &mesh.clone().transformed_by(transform)
        } else {
            mesh
        };
        let collider =
            Collider::convex_decomposition_from_mesh_with_config(mesh, &VHACD_PARAMETERS)
                .ok_or_else(|| anyhow!("convex decomposition failed"))?;

        Ok(Self::collider(uuid, material, collider))
    }

    pub fn collider(uuid: &str, material: VoxelMaterial, collider: Collider) -> Self {
        let aabb = collider
            .aabb(Vec3::ZERO, Rotation::default())
            .grow(Vec3::splat(VOXEL_REAL_SIZE));
        let chunks = ChunksAABB::from_world_aabb((aabb.min, aabb.max), 0);

        Self::Collider {
            uuid: uuid.to_owned(),
            collider,
            material,
            chunks,
        }
    }

    //
    // Sampling
    //

    fn sample_curve(&self, point: Vec3) -> VoxelSample {
        let TerrainBrush::Curve {
            curve,
            radius,
            material,
            ..
        } = self
        else {
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
        let TerrainBrush::Collider {
            collider, material, ..
        } = self
        else {
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

//
// Plugin
//

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
