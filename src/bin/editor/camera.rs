use core::f32;

use bevy::{prelude::*, render::view::RenderLayers};
use bevy_trackball::{
    prelude::{Bound, Clamp, Scope},
    TrackballCamera, TrackballController, TrackballInput, TrackballVelocity, TrackballWheelUnit,
};
use mines::render_layer;
use nalgebra::{Point3, Vector3};
use transform_gizmo_bevy::GizmoCamera;

use crate::state::{EditorMode, EditorState, EditorViewMode};

#[derive(Component)]
pub struct AllowOrbit(pub bool);

pub fn on_change_mode(
    mut commands: Commands,
    state: Res<EditorState>,
    trackball: Option<
        Single<(
            Entity,
            &mut TrackballController,
            &mut TrackballCamera,
            &mut AllowOrbit,
        )>,
    >,
) {
    let Some(trackball) = trackball else {
        return;
    };
    let (entity, mut controller, mut camera, mut allow_orbit) = trackball.into_inner();
    let (d, mut target, up, eps) = (16.0, Point3::origin(), &Vector3::y_axis(), f32::EPSILON);

    let mut block_orbit = false;

    match state.mode() {
        EditorMode::Tunnels => match state.view {
            EditorViewMode::Editor => {
                camera.scope.set_ortho(true);
                camera.frame.set_target(target);
                camera.frame.set_eye(&Point3::new(0.0, d, -eps), up);
                block_orbit = true;
            }
            EditorViewMode::Preview => {
                target.y += 8.0;
                camera.scope.set_ortho(false);
                camera.frame.set_target(target);
                camera.frame.set_eye(&Point3::new(0.0, d, -d / 2.0), up);
            }
        },
        EditorMode::Rooms => {
            camera.scope.set_ortho(false);
            camera.frame.set_target(target);
            camera.frame.set_eye(&Point3::new(-d, d / 2.0, -d), up);
        }
    }

    controller.input.orbit_button = if block_orbit {
        None
    } else {
        if controller.input.slide_button == Some(MouseButton::Right) {
            Some(MouseButton::Middle)
        } else {
            Some(MouseButton::Right)
        }
    };

    commands.entity(entity).insert(match state.view {
        EditorViewMode::Editor => {
            RenderLayers::from_layers(&[render_layer::WORLD, render_layer::EDITOR])
        }
        EditorViewMode::Preview => {
            RenderLayers::from_layers(&[render_layer::WORLD, render_layer::EDITOR_PREVIEW])
        }
    });
    camera.reset = camera.frame;
    camera.clamp = clamp(!block_orbit);
    allow_orbit.0 = !block_orbit;
}

fn clamp(allow_orbit: bool) -> Option<Box<dyn Clamp<f32>>> {
    if allow_orbit {
        return None;
    };

    Some(Box::new({
        let mut bound = Bound::default();
        bound.min_up = Point3::origin();
        bound.max_up = Point3::origin();
        bound
    }))
}

pub fn setup(mut commands: Commands) {
    let mut scope = Scope::default();
    scope.set_ortho(true);

    let mut controller = TrackballController::default();
    controller.input = TrackballInput {
        velocity: TrackballVelocity::default(),
        wheel_unit: TrackballWheelUnit::default(),

        focus: true,

        gamer_key: None,

        ortho_key: None,
        reset_key: None,

        first_key: None,
        first_button: None,
        first_left_key: None,
        first_right_key: None,
        first_up_key: None,
        first_down_key: None,

        orbit_button: Some(MouseButton::Middle),
        screw_left_key: None,
        screw_right_key: None,
        orbit_left_key: None,
        orbit_right_key: None,
        orbit_up_key: None,
        orbit_down_key: None,

        slide_button: Some(MouseButton::Right),
        slide_up_key: None,
        slide_down_key: None,
        slide_left_key: None,
        slide_right_key: None,
        slide_far_key: None,
        slide_near_key: None,

        scale_in_key: None,
        scale_out_key: None,
    };

    commands.spawn((
        RenderLayers::from_layers(&[render_layer::WORLD, render_layer::EDITOR]),
        AllowOrbit(false),
        controller,
        TrackballCamera::look_at(Vec3::ZERO, Vec3::new(0.00, 16.0, f32::EPSILON), Vec3::Y)
            .with_scope(scope)
            .with_blend(0.0),
        Camera3d::default(),
        GizmoCamera,
        PointLight {
            intensity: 300_000_000.0,
            range: 2048.0,
            color: Color::WHITE.into(),
            ..default()
        },
    ));
}
