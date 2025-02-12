// References:
// - https://www.peroxide.dk/papers/collision/collision.pdf
// - https://github.com/Jondolf/avian/blob/45d9f8fa16c28530e77d1c96e7d600cbf2b46fad/crates/avian3d/examples/collide_and_slide_3d/plugin.rs
// - https://github.com/nicholas-maltbie/OpenKCC/blob/a1a30ed7f7722ea82a1df6bd01849e0bfde6abf4/Assets/Samples/SimplifiedDemoKCC/Scripts/SimplifiedKCC.cs
// - https://github.com/Desine-Unity/collide-and-slide/blob/main/Runtime/CollideAndSlide.cs

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{
    quakeish::{air_move, ground_move},
    PlayerCamera, PlayerKeybinds, Section,
};

const MAX_SLOPE_DEGREES: f32 = 55.0;
const GROUND_DISTANCE: f32 = 0.15;
const MAX_BOUNCES: u32 = 4;
const SKIN: f32 = 0.01;
const JUMP_FORCE: f32 = 16.0;
const JUMP_BUFFER_DISTANCE: f32 = 1.0;
const GRAVITY: f32 = 64.0;

#[derive(Default, Component)]
pub struct PlayerMotion {
    pub grounded: bool,
    pub ground_normal: Option<Vec3>,
    pub ground_distance: Option<f32>,
    pub landed_time: f64,
    pub gravity: Vec3,
    pub movement: Vec3,
}

#[derive(Resource, Default)]
pub struct PlayerInput {
    /// Commanded movement direction, local XZ plane.
    direction: Vec2,
    crouch: bool,
    sprint: bool,

    actions: Vec<PlayerAction>,
}
impl PlayerInput {
    pub fn action(&mut self, action: PlayerAction) {
        if !self.actions.contains(&action) {
            self.actions.push(action);
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum PlayerAction {
    Jump,
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
                perform_actions,
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
    state: Option<Single<&PlayerMotion>>,
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

    if let Some(jump) = &binds.jump {
        if let Some(state) = state {
            if let Some(ground_distance) = state.ground_distance {
                if jump.just_pressed(&keyboard, &mouse) && ground_distance <= JUMP_BUFFER_DISTANCE {
                    input.action(PlayerAction::Jump);
                }
            }
        }
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

    //state.velocity *= 0.999 / ;
}

fn perform_actions(mut input: ResMut<PlayerInput>, state: Option<Single<&mut PlayerMotion>>) {
    let Some(mut state) = state else {
        return;
    };

    input.actions.retain(|action| {
        let mut consumed = false;
        let mut consume = || consumed = true;

        match action {
            PlayerAction::Jump => {
                if state.grounded {
                    state.gravity.y += JUMP_FORCE;
                    consume();
                }
            }
        };

        !consumed
    });
}

fn snap_to_ground(
    time: Res<Time>,
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
        &ShapeCastConfig::from_max_distance(JUMP_BUFFER_DISTANCE),
        &SpatialQueryFilter::from_excluded_entities(vec![entity]),
    );
    let Some(hit) = shapecast else {
        state.grounded = false;
        state.ground_distance = None;
        return;
    };

    state.ground_distance = Some(hit.distance);

    if hit.distance > GROUND_DISTANCE {
        state.grounded = false;
        return;
    }

    let prev_grounded = state.grounded;

    let angle = hit.normal1.angle_between(Vec3::Y);
    state.grounded = angle < MAX_SLOPE_DEGREES.to_radians();
    state.ground_normal = Some(hit.normal1);

    if !state.grounded {
        return;
    }

    if state.gravity.y <= 0.0 {
        transform.translation.y -= hit.distance - SKIN;
    }
    state.gravity.y = state.gravity.y.max(0.0);

    if !prev_grounded {
        state.landed_time = time.elapsed_secs_f64();
    }
}

fn movement_pass(
    time: Res<Time>,
    input: Res<PlayerInput>,
    spatial_query: SpatialQuery,
    player: Option<Single<(Entity, &mut Transform, &Section, &mut PlayerMotion)>>,
    camera: Option<Single<&GlobalTransform, With<PlayerCamera>>>,
) {
    let Some(player) = player else {
        return;
    };
    let Some(camera) = camera else {
        return;
    };

    let (entity, mut transform, section, mut state) = player.into_inner();

    let mut wishdir = Vec3::new(input.direction.x, 0.0, input.direction.y);

    let yaw = camera.rotation().to_euler(EulerRot::YXZ).0;
    let rotation = Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.0));
    wishdir = rotation.transform_point(wishdir);

    if state.grounded {
        ground_move(wishdir, state.landed_time, &mut state.movement, &time);
    } else {
        air_move(wishdir, &mut state.movement, &time);
    }

    collide_and_slide(
        Pass::Movement,
        section,
        state.grounded,
        &mut state.movement,
        &mut transform.translation,
        &spatial_query,
        &SpatialQueryFilter::from_excluded_entities(vec![entity]),
        &time,
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

    let mut gravity = Vec3::NEG_Y * GRAVITY * time.delta_secs();
    if state.grounded {
        gravity *= 0.01;
    }
    state.gravity += gravity;

    collide_and_slide(
        Pass::Gravity,
        section,
        state.grounded,
        &mut state.gravity,
        &mut transform.translation,
        &spatial_query,
        &filter,
        &time,
    );

    depenetrate(&spatial_query, &filter, &section.collider(), &mut transform);
}

fn collide_and_slide(
    pass: Pass,
    section: &Section,
    grounded: bool,
    velocity: &mut Vec3,
    position: &mut Vec3,
    spatial_query: &SpatialQuery,
    filter: &SpatialQueryFilter,
    time: &Res<Time>,
) {
    let mut time_velocity = *velocity * time.delta_secs();
    let initial_velocity = time_velocity * time.delta_secs();

    for _ in 0..MAX_BOUNCES {
        let Ok((direction, distance)) = Dir3::new_and_length(time_velocity) else {
            break;
        };

        let shapecast = spatial_query.cast_shape(
            &section.inflated(-SKIN).collider_centered(),
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
            *position += time_velocity;
            break;
        };

        let snap_to_surface = direction * (hit.distance - SKIN);
        let angle_of_normal = hit.normal1.angle_between(Vec3::Y);

        if snap_to_surface.length() <= SKIN {
            time_velocity = Vec3::ZERO;
            *velocity = Vec3::ZERO;
        }

        time_velocity = if angle_of_normal <= MAX_SLOPE_DEGREES.to_radians() {
            if pass == Pass::Gravity {
                snap_to_surface
            } else {
                time_velocity.length() * time_velocity.project_onto_normalized(hit.normal1)
            }
        } else {
            let horizontal = |v: Vec3| Vec3::new(v.x, 0.0, v.z);
            let scale = 1.0 - horizontal(hit.normal1).dot(-horizontal(initial_velocity));

            *velocity *= scale;

            if grounded && pass != Pass::Gravity {
                horizontal(time_velocity).project_onto_normalized(horizontal(hit.normal1)) * scale
            } else {
                time_velocity.length() * time_velocity.project_onto_normalized(hit.normal1) * scale
            }
        };

        *position += time_velocity;
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
        transform.translation += hit.normal1 * (hit.distance + SKIN);
    }
}
