use std::f32::consts::PI;

use bevy::{prelude::*, render::view::RenderLayers, scene::SceneInstance};

use crate::render_layer;

pub const VIEWMODEL_FOV: f32 = 65.0;

#[derive(Component, Default)]
pub struct ViewModel {
    pub yaw: f32,
    pub pitch: f32,
    pub offset: Vec3,
}

#[derive(Component)]
pub struct NeedsRenderLayers(pub RenderLayers);

#[derive(Component, Default)]
pub struct ViewModelCamera;

pub struct ViewModelPlugin;

impl Plugin for ViewModelPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (add_required_components, insert_render_layers));
        app.add_systems(PostUpdate, inertia);
    }
}

// fn inertia(
//     time: Res<Time>,
//     parents: Query<&GlobalTransform, Without<ViewModelCamera>>,
//     mut viewmodel_cameras: Query<
//         (&mut ViewModelCamera, &mut Transform, &Parent),
//         With<ViewModelCamera>,
//     >,
// ) {
//     const CIRCLE: f32 = PI * 2.0;
//     fn interpolate_angle(a: f32, b: f32, t: f32) -> f32 {
//         let shortest = ((b - a) % CIRCLE + CIRCLE * 1.5) % CIRCLE - CIRCLE / 2.0;
//         a + shortest * t
//     }

//     fn interpolate_angle_clamped(a: f32, b: f32, t: f32) -> f32 {
//         const CLAMP: f32 = (PI / 180.0) * 20.0;
//         const CIRCLE: f32 = PI * 2.0;

//         let shortest = ((b - a) % CIRCLE + CIRCLE * 1.5) % CIRCLE - CIRCLE / 2.0;
//         let mut result = a + shortest * t;

//         if shortest.abs() > CLAMP {
//             if shortest < 0.0 {
//                 result = b + CLAMP;
//             } else {
//                 result = b - CLAMP;
//             }
//         }

//         result
//     }

//     viewmodel_cameras
//         .iter_mut()
//         .for_each(|(mut camera, mut transform, parent)| {
//             let Ok(parent) = parents.get(**parent) else {
//                 return;
//             };

//             let (parent_yaw, parent_pitch, _) = parent.rotation().to_euler(EulerRot::YXZ);

//             let t = (time.delta_secs() * 1.0).clamp(0.0, 1.0);

//             camera.yaw = interpolate_angle(camera.yaw, parent_yaw, t);
//             camera.pitch = interpolate_angle(camera.pitch, parent_pitch, t);

//             println!("{}  {}", camera.yaw, parent_yaw);

//             // let mul_yaw = 0.15;
//             // let mul_pitch = 0.3;
//             // // println!("{}", offset);
//             // // *transform = Transform::from_translation(viewmodel.offset + offset)
//             // //     .looking_at(Vec3::NEG_Z * 100.0, Vec3::Y);

//             transform.rotation = Quat::from_euler(EulerRot::YXZ, camera.yaw, camera.pitch, 0.0);
//         });
// }

fn inertia(
    time: Res<Time>,
    parents: Query<&GlobalTransform, Without<ViewModel>>,
    mut viewmodels: Query<(&mut ViewModel, &mut Transform, &Parent), With<ViewModel>>,
) {
    const CIRCLE: f32 = PI * 2.0;
    fn interpolate_angle(a: f32, b: f32, t: f32) -> f32 {
        let shortest = ((b - a) % CIRCLE + CIRCLE * 1.5) % CIRCLE - CIRCLE / 2.0;
        a + shortest * t.clamp(0.0, 1.0)
    }

    fn interpolate_angle_clamped(a: f32, b: f32, t: f32) -> f32 {
        const CLAMP: f32 = (PI / 180.0) * 20.0;
        const CIRCLE: f32 = PI * 2.0;

        let shortest = ((b - a) % CIRCLE + CIRCLE * 1.5) % CIRCLE - CIRCLE / 2.0;
        let mut result = a + shortest * t;

        if shortest.abs() > CLAMP {
            if shortest < 0.0 {
                result = b + CLAMP;
            } else {
                result = b - CLAMP;
            }
        }

        result
    }

    viewmodels
        .iter_mut()
        .for_each(|(mut viewmodel, mut transform, parent)| {
            let Ok(parent) = parents.get(**parent) else {
                return;
            };

            let (parent_yaw, parent_pitch, _) = parent.rotation().to_euler(EulerRot::YXZ);

            let t = time.delta_secs() * 24.0;

            viewmodel.yaw = interpolate_angle(viewmodel.yaw, parent_yaw, t);
            viewmodel.pitch = interpolate_angle(viewmodel.pitch, parent_pitch, t);

            *transform = Transform::from_rotation(Quat::from_euler(
                EulerRot::YXZ,
                interpolate_angle(viewmodel.yaw, parent_yaw, 0.5) - parent_yaw,
                viewmodel.pitch - parent_pitch,
                0.0,
            ));
        });
}

fn add_required_components(
    mut commands: Commands,
    viewmodel_cameras: Query<Entity, Added<ViewModelCamera>>,
) {
    viewmodel_cameras.iter().for_each(|entity| {
        let mut commands = commands.entity(entity);
        commands.insert((
            Transform::default(),
            Camera3d::default(),
            Camera {
                order: 1,
                ..default()
            },
            Projection::from(PerspectiveProjection {
                fov: VIEWMODEL_FOV.to_radians(),
                ..default()
            }),
            RenderLayers::layer(render_layer::VIEW_MODEL),
        ));
    });
}

// HACK https://github.com/bevyengine/bevy/issues/5183
fn insert_render_layers(
    mut commands: Commands,
    scenes: Query<(Entity, &SceneInstance, &NeedsRenderLayers)>,
    scene_spawner: Res<SceneSpawner>,
) {
    scenes
        .iter()
        .for_each(|(entity, scene, needs_render_layers)| {
            if !scene_spawner.instance_is_ready(**scene) {
                return;
            }

            scene_spawner
                .iter_instance_entities(**scene)
                .for_each(|entity| {
                    commands
                        .entity(entity)
                        .insert(needs_render_layers.0.clone());
                });

            commands.entity(entity).remove::<NeedsRenderLayers>();
        });
}
