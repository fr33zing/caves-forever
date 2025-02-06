use avian3d::prelude::*;
use bevy::{
    prelude::*,
    tasks::{block_on, futures_lite::future, AsyncComputeTaskPool, Task},
};
use curvo::prelude::{NurbsCurve3D, Tessellation};
use nalgebra::{Const, Point3};

use super::{
    chunk::ChunksAABB,
    consts::{TUNNEL_VHACD_PARAMETERS, VOXEL_REAL_SIZE},
    utility::safe_vhacd,
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
        sequence: usize,
        material: VoxelMaterial,
        points: Vec<Point3<f32>>,
        radius: f32,
    },
    Sweep {
        uuid: String,
        sequence: usize,
        material: VoxelMaterial,
        rail: Vec<Point3<f32>>,
        profile: ProfileRamp,
    },
    Mesh {
        uuid: String,
        sequence: usize,
        material: VoxelMaterial,
        mesh: Mesh,
        transform: Transform,
        vhacd_parameters: VhacdParameters,
    },
}

#[derive(Component, Clone)]
pub enum TerrainBrush {
    Curve {
        uuid: String,
        sequence: usize,
        curve: NurbsCurve3D<f32>,
        radius: f32,
        material: VoxelMaterial,
        chunks: ChunksAABB,
    },
    Collider {
        uuid: String,
        sequence: usize,
        collider: Collider,
        material: VoxelMaterial,
        chunks: ChunksAABB,
        transform: Transform,
    },
}

impl TerrainBrushRequest {
    pub fn process(self) -> TerrainBrush {
        match self {
            TerrainBrushRequest::Curve {
                uuid,
                sequence,
                material,
                points,
                radius,
            } => TerrainBrush::curve(&uuid, sequence, material, &points, radius),
            TerrainBrushRequest::Sweep {
                uuid,
                sequence,
                material,
                rail,
                profile,
            } => TerrainBrush::sweep(&uuid, sequence, material, &rail, &profile).unwrap_or_else(
                |_| {
                    // TODO dynamic fallback curve radius
                    TerrainBrush::curve(&uuid, sequence, VoxelMaterial::Invalid, &rail, 4.0)
                },
            ),
            TerrainBrushRequest::Mesh {
                uuid,
                sequence,
                material,
                mesh,
                transform,
                vhacd_parameters,
            } => TerrainBrush::mesh(
                &uuid,
                sequence,
                material,
                &mesh,
                Some(transform),
                &vhacd_parameters,
            )
            .unwrap_or_else(|_| {
                // TODO dynamic fallback sphere radius
                TerrainBrush::collider(
                    &uuid,
                    sequence,
                    VoxelMaterial::Invalid,
                    Collider::sphere(2.0 * transform.scale.max_element()),
                    transform,
                )
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

    pub fn sequence(&self) -> usize {
        match self {
            TerrainBrush::Curve { sequence, .. } => *sequence,
            TerrainBrush::Collider { sequence, .. } => *sequence,
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

    pub fn curve(
        uuid: &str,
        sequence: usize,
        material: VoxelMaterial,
        points: &[Point3<f32>],
        radius: f32,
    ) -> Self {
        let curve = NurbsCurve3D::<f32>::try_interpolate(points, 3).unwrap();
        let samples = curve.tessellate(Some(1e-8));
        let aabb = curve_bounding_box(&samples);
        let chunks = ChunksAABB::from_world_aabb(aabb, 1);

        Self::Curve {
            uuid: uuid.to_owned(),
            sequence,
            curve,
            radius,
            material,
            chunks,
        }
    }

    pub fn sweep(
        uuid: &str,
        sequence: usize,
        material: VoxelMaterial,
        rail: &[Point3<f32>],
        profile: &ProfileRamp,
    ) -> anyhow::Result<Self> {
        let rail = NurbsCurve3D::<f32>::try_interpolate(rail, 3)?;
        let mesh = sweep_zero_twist_filled::<Const<4>>(profile, &rail, Some(4))?;

        Self::mesh(
            uuid,
            sequence,
            material,
            &mesh,
            None,
            &TUNNEL_VHACD_PARAMETERS,
        )
    }

    pub fn mesh(
        uuid: &str,
        sequence: usize,
        material: VoxelMaterial,
        mesh: &Mesh,
        transform: Option<Transform>,
        vhacd_parameters: &VhacdParameters,
    ) -> anyhow::Result<Self> {
        let mesh = if let Some(transform) = transform {
            &mesh.clone().scaled_by(transform.scale)
        } else {
            mesh
        };
        let collider = safe_vhacd(&mesh, &vhacd_parameters)?;

        Ok(Self::collider(
            uuid,
            sequence,
            material,
            collider,
            transform.unwrap_or_else(|| Transform::default()),
        ))
    }

    pub fn collider(
        uuid: &str,
        sequence: usize,
        material: VoxelMaterial,
        collider: Collider,
        transform: Transform,
    ) -> Self {
        let aabb = collider
            .aabb(Vec3::ZERO, Rotation::default())
            .grow(Vec3::splat(VOXEL_REAL_SIZE));
        let chunks = ChunksAABB::from_world_aabb(
            (
                aabb.min + transform.translation,
                aabb.max + transform.translation,
            ),
            0,
        );

        Self::Collider {
            uuid: uuid.to_owned(),
            sequence,
            collider,
            material,
            chunks,
            transform,
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
            collider,
            material,
            transform,
            ..
        } = self
        else {
            panic!("wrong sample function");
        };

        // is_inside from project_point is unreliable so we need to do it twice
        let (position, rotation) = (transform.translation, transform.rotation);
        let (closest, _) = collider.project_point(position, rotation, point, false);
        let (closest_solid, _) = collider.project_point(position, rotation, point, true);
        let mut distance = point.distance(closest);
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

fn process_brushes(
    mut commands: Commands,
    requests: Query<(Option<&Parent>, Entity, &TerrainBrushRequest)>,
) {
    let task_pool = AsyncComputeTaskPool::get();

    requests
        .iter()
        .for_each(|(parent, request_entity, request)| {
            let request = request.clone();
            let task = task_pool.spawn(async move { request.process() });

            let task_entity = commands.spawn(TerrainBrushTask(task)).id();
            if let Some(parent) = parent {
                let mut commands = commands.entity(parent.get());
                commands.remove_children(&[request_entity]);
                commands.add_child(task_entity);
            }
            commands.entity(request_entity).despawn();
        });
}

fn receive_brushes(
    mut commands: Commands,
    mut tasks: Query<(Option<&Parent>, Entity, &mut TerrainBrushTask)>,
) {
    for (parent, task_entity, mut task) in tasks.iter_mut() {
        let status = block_on(future::poll_once(&mut task.0));

        let Some(brush) = status else {
            continue;
        };

        let brush_entity = commands.spawn(brush).id();
        if let Some(parent) = parent {
            let mut commands = commands.entity(parent.get());
            commands.remove_children(&[task_entity]);
            commands.add_child(brush_entity);
        }
        commands.entity(task_entity).despawn();
    }
}
