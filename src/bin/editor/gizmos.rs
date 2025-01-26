use bevy::{math::Vec3A, picking::backend::ray::RayMap, prelude::*};
use transform_gizmo_bevy::{
    Color32, EnumSet, GizmoHotkeys, GizmoMode, GizmoOptions, GizmoOrientation, GizmoTarget,
    GizmoVisuals, TransformGizmoPlugin,
};

pub struct EditorGizmosPlugin;

#[derive(Component)]
pub struct Pickable(pub Option<EnumSet<GizmoMode>>, pub Option<GizmoOrientation>);

#[derive(Component)]
pub struct ConnectionPlane;

#[derive(Component)]
pub struct ConnectionPoint;

#[derive(Component)]
pub struct ConnectedPath;

impl Plugin for EditorGizmosPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((MeshPickingPlugin, TransformGizmoPlugin));
        app.insert_resource(MeshPickingSettings {
            require_markers: true,
            ray_cast_visibility: RayCastVisibility::VisibleInView,
        });
        app.insert_resource(GizmoOptions {
            visuals: GizmoVisuals {
                x_color: Color32::from_rgb(250, 70, 70),
                y_color: Color32::from_rgb(70, 250, 70),
                z_color: Color32::from_rgb(70, 70, 250),
                inactive_alpha: 0.2,
                highlight_alpha: 0.6,
                stroke_width: 3.0,
                gizmo_size: 70.0,
                ..default()
            },
            hotkeys: Some(GizmoHotkeys::default()),
            ..default()
        });

        app.add_systems(
            Update,
            (pick, draw_connection_planes, draw_connection_points),
        );
    }
}

fn pick(
    mut commands: Commands,
    mut ray_cast: MeshRayCast,
    ray_map: Res<RayMap>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut gizmo_options: ResMut<GizmoOptions>,
    pickables: Query<(Entity, &Pickable)>,
    gizmo_targets: Query<(Entity, &GizmoTarget)>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }
    if gizmo_targets.iter().any(|(_, target)| target.is_focused()) {
        return;
    }

    let mut miss = true;

    for (_, ray) in ray_map.iter() {
        let Some((entity, _)) = ray_cast.cast_ray(*ray, &RayCastSettings::default()).first() else {
            continue;
        };
        let Ok((entity, pickable)) = pickables.get(*entity) else {
            continue;
        };

        gizmo_targets.iter().for_each(|(entity, _)| {
            commands.entity(entity).remove::<GizmoTarget>();
        });
        commands.entity(entity).insert(GizmoTarget::default());

        gizmo_options.gizmo_modes = pickable.0.unwrap_or_else(|| GizmoMode::all());
        gizmo_options.gizmo_orientation = pickable.1.unwrap_or_else(|| GizmoOrientation::default());

        miss = false;
        break;
    }

    if miss {
        gizmo_targets.iter().for_each(|(entity, _)| {
            commands.entity(entity).remove::<GizmoTarget>();
        });
    }
}

fn draw_connection_planes(
    mut gizmos: Gizmos,
    planes: Query<(&Transform, Option<&GizmoTarget>), With<ConnectionPlane>>,
) {
    planes.iter().for_each(
        |(
            Transform {
                translation,
                rotation,
                scale,
            },
            selected,
        )| {
            let color = if selected.is_some() {
                Color::srgb(0.0, 1.0, 1.0)
            } else {
                Color::srgb(1.0, 1.0, 1.0)
            };

            let isometry = Isometry3d {
                translation: Vec3A::new(translation.x, translation.y, translation.z),
                rotation: *rotation
                    * Quat::from_euler(EulerRot::XYZ, 90.0_f32.to_radians(), 0.0, 0.0),
            };
            gizmos.rect(isometry, scale.xz(), color);

            let t = Transform::from_translation(*translation).with_rotation(*rotation);
            let end = t.transform_point(Vec3::Y * 2.0);
            gizmos.arrow(*translation, end, color);
        },
    );
}

fn draw_connection_points(
    mut gizmos: Gizmos,
    camera: Single<&Transform, With<Camera3d>>,
    points: Query<(&GlobalTransform, Option<&Pickable>), With<ConnectionPoint>>,
) {
    points.iter().for_each(|(transform, pickable)| {
        if pickable.is_some() {
            return;
        }

        let color = Color::srgb(0.7, 0.7, 0.7);
        let translation = transform.translation();
        let isometry = Isometry3d {
            translation: translation.into(),
            rotation: Transform::from_translation(translation)
                .looking_at(camera.translation, Vec3::Y)
                .rotation,
        };

        gizmos.circle(isometry, 0.5, color);
    });
}
