// References:
// - https://www.peroxide.dk/papers/collision/collision.pdf
// - https://github.com/Jondolf/avian/blob/45d9f8fa16c28530e77d1c96e7d600cbf2b46fad/crates/avian3d/examples/collide_and_slide_3d/plugin.rs
// - https://github.com/nicholas-maltbie/OpenKCC/blob/a1a30ed7f7722ea82a1df6bd01849e0bfde6abf4/Assets/Samples/SimplifiedDemoKCC/Scripts/SimplifiedKCC.cs

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{PlayerCamera, PlayerKeybinds, Section};

const MAX_SLOPE_DEGREES: f32 = 55.0;
const GROUND_DISTANCE: f32 = 0.15;
const MAX_BOUNCES: u32 = 1;
const COLLISION_EPSILON: f32 = 0.01;
const DEPENETRATION_EPSILON: f32 = 0.01;
const G: f32 = 9.81;
const PLAYER_MASS: f32 = 80.0;
const JUMP_FORCE: f32 = 1.0;

#[derive(Component)]
pub struct PlayerMotion {
    pub grounded: bool,
    pub ground_normal: Option<Vec3>,
    pub velocity: Vec3,
}

impl Default for PlayerMotion {
    fn default() -> Self {
        Self {
            grounded: true,
            ground_normal: None,
            velocity: Vec3::ZERO,
        }
    }
}

#[derive(Resource, Default)]
pub struct PlayerInput {
    /// Commanded movement direction, local XZ plane.
    direction: Vec2,
    jump: bool,
    crouch: bool,
    sprint: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Pass {
    Movement,
    Gravity,
}

pub struct PlayerMotionPlugin;

impl Plugin for PlayerMotionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerInput>();
        app.add_systems(
            Update,
            (
                process_input,
                drag,
                jump,
                snap_to_ground,
                movement_pass,
                gravity_pass,
            )
                .chain(),
        );
    }
}

fn process_input(
    mut input: ResMut<PlayerInput>,
    binds: Res<PlayerKeybinds>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
) {
    input.direction = Vec2::ZERO;

    if let Some(forward) = &binds.forward {
        if forward.pressed(&keyboard, &mouse) {
            input.direction += Vec2::NEG_Y;
        }
    }
    if let Some(backward) = &binds.backward {
        if backward.pressed(&keyboard, &mouse) {
            input.direction += Vec2::Y;
        }
    }
    if let Some(left) = &binds.left {
        if left.pressed(&keyboard, &mouse) {
            input.direction += Vec2::NEG_X;
        }
    }
    if let Some(right) = &binds.right {
        if right.pressed(&keyboard, &mouse) {
            input.direction += Vec2::X;
        }
    }

    if input.direction.length() > 0.0 {
        input.direction = input.direction.normalize();
    }

    input.jump = if let Some(jump) = &binds.jump {
        jump.just_pressed(&keyboard, &mouse)
    } else {
        false
    };

    if let Some(crouch) = &binds.crouch {
        if crouch.pressed(&keyboard, &mouse) {
            input.crouch = true;
        }
    }
    if let Some(sprint) = &binds.sprint {
        if sprint.pressed(&keyboard, &mouse) {
            input.sprint = true;
        }
    }
}

fn drag(state: Option<Single<&mut PlayerMotion>>) {
    let Some(mut state) = state else {
        return;
    };

    //state.velocity *= 0.999;
}

fn jump(input: Res<PlayerInput>, state: Option<Single<&mut PlayerMotion>>) {
    if !input.jump {
        return;
    }

    let Some(mut state) = state else {
        return;
    };

    state.velocity.y += JUMP_FORCE;
}

fn snap_to_ground(
    spatial_query: SpatialQuery,
    player: Option<Single<(Entity, &mut Transform, &Section, &mut PlayerMotion)>>,
) {
    let Some(player) = player else {
        return;
    };

    let (entity, mut transform, section, mut state) = player.into_inner();

    let bottom = transform.translation;
    let shapecast = spatial_query.cast_shape(
        &section.collider(),
        bottom,
        default(),
        Dir3::NEG_Y,
        &ShapeCastConfig::from_max_distance(GROUND_DISTANCE),
        &SpatialQueryFilter::from_excluded_entities(vec![entity]),
    );
    let Some(hit) = shapecast else {
        state.grounded = false;
        return;
    };

    let angle = hit.normal1.angle_between(Vec3::Y);
    state.grounded = angle < MAX_SLOPE_DEGREES.to_radians();
    state.ground_normal = Some(hit.normal1);

    if state.grounded {
        if state.velocity.y <= 0.0 {
            transform.translation.y -= hit.distance - COLLISION_EPSILON;
        }

        state.velocity.y = state.velocity.y.max(0.0);
    }
}

fn movement_pass(
    time: Res<Time>,
    input: Res<PlayerInput>,
    spatial_query: SpatialQuery,
    player: Option<Single<(Entity, &mut Transform, &Section)>>,
    camera: Option<Single<&GlobalTransform, With<PlayerCamera>>>,
) {
    let Some(player) = player else {
        return;
    };
    let Some(camera) = camera else {
        return;
    };

    let (entity, mut transform, section) = player.into_inner();

    let mut velocity = Vec3::new(input.direction.x, 0.0, input.direction.y);
    velocity *= 32.0 * time.delta_secs();

    let yaw = camera.rotation().to_euler(EulerRot::YXZ).0;
    let rotation = Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.0));
    velocity = rotation.transform_point(velocity);

    collide_and_slide(
        Pass::Movement,
        section,
        velocity,
        &mut transform.translation,
        &spatial_query,
        &SpatialQueryFilter::from_excluded_entities(vec![entity]),
    );
}

fn gravity_pass(
    time: Res<Time>,
    spatial_query: SpatialQuery,
    player: Option<Single<(Entity, &mut Transform, &Section, &mut PlayerMotion)>>,
) {
    let Some(player) = player else {
        return;
    };

    let (entity, mut transform, section, mut state) = player.into_inner();

    let filter = SpatialQueryFilter::from_excluded_entities(vec![entity]);

    let mut gravity = Vec3::NEG_Y * G * time.delta_secs();
    if state.grounded {
        gravity *= 0.01;
    }
    state.velocity += gravity;

    collide_and_slide(
        Pass::Gravity,
        section,
        state.velocity.clone(),
        &mut transform.translation,
        &spatial_query,
        &filter,
    );

    depenetrate(&spatial_query, &filter, &section.collider(), &mut transform);

    println!("{}", state.velocity);
}

fn calculate_sliding_velocity(planes: &mut Vec<Vec3>, normal: Vec3, velocity: Vec3) -> Vec3 {
    planes.push(normal);
    let mut result = velocity.reject_from(normal);

    if planes.len() > 1 {
        result = planes.windows(2).fold(result, |acc, plane_pair| {
            acc.project_onto(plane_pair[0].cross(plane_pair[1]))
        });
    }

    result
}

fn collide_and_slide(
    pass: Pass,
    section: &Section,
    velocity: Vec3,
    position: &mut Vec3,
    spatial_query: &SpatialQuery,
    filter: &SpatialQueryFilter,
) {
    let mut velocity = velocity;
    let mut collision_planes = Vec::with_capacity(MAX_BOUNCES as usize);

    for _ in 0..MAX_BOUNCES {
        let Ok((direction, distance)) = Dir3::new_and_length(velocity) else {
            break;
        };

        let shapecast = spatial_query.cast_shape(
            &section.collider_centered(),
            section.center(*position),
            default(),
            direction,
            &ShapeCastConfig {
                max_distance: distance,
                target_distance: 0.0,
                compute_contact_on_penetration: true,
                ignore_origin_penetration: true,
            },
            filter,
        );
        let Some(hit) = shapecast else {
            *position += velocity;
            break;
        };

        let safe_distance = (hit.distance - COLLISION_EPSILON).max(0.0);
        let safe_movement = velocity * safe_distance;

        *position += safe_movement;
        velocity -= safe_movement;

        let too_steep = hit.normal1.angle_between(Vec3::Y) > MAX_SLOPE_DEGREES.to_radians();

        if pass == Pass::Gravity && !too_steep {
            break;
        }

        let normal = if pass == Pass::Movement && too_steep {
            (hit.normal1 * Vec3::new(1.0, 0.0, 1.0)).normalize()
        } else {
            hit.normal1
        };

        velocity = calculate_sliding_velocity(&mut collision_planes, normal, velocity);
    }
}

fn depenetrate(
    spatial_query: &SpatialQuery,
    filter: &SpatialQueryFilter,
    collider: &Collider,
    transform: &mut Transform,
) {
    let config = ShapeCastConfig {
        max_distance: 0.0,
        target_distance: 0.0,
        compute_contact_on_penetration: true,
        ignore_origin_penetration: false,
    };

    let hit = spatial_query.cast_shape(
        collider,
        transform.translation,
        transform.rotation,
        Dir3::NEG_Y,
        &config,
        filter,
    );

    if let Some(hit) = hit {
        transform.translation += hit.normal1 * (hit.distance + DEPENETRATION_EPSILON);
    }
}
