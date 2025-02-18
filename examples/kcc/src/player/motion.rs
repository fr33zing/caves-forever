// References:
// - https://www.peroxide.dk/papers/collision/collision.pdf
// - https://github.com/Jondolf/avian/blob/45d9f8fa16c28530e77d1c96e7d600cbf2b46fad/crates/avian3d/examples/collide_and_slide_3d/plugin.rs
// - https://github.com/nicholas-maltbie/OpenKCC/blob/a1a30ed7f7722ea82a1df6bd01849e0bfde6abf4/Assets/Samples/SimplifiedDemoKCC/Scripts/SimplifiedKCC.cs
// - https://github.com/Desine-Unity/collide-and-slide/blob/main/Runtime/CollideAndSlide.cs

// TODO add an expiration time to buffered actions

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{
    quakeish::{air_move, ground_move},
    PlayerCamera, PlayerKeybinds, Section,
};

const MAX_SLOPE_DEGREES: f32 = 55.0;
const GROUND_DISTANCE: f32 = 0.1;
const MAX_BOUNCES: u32 = 3;
const SKIN: f32 = 0.005;
const JUMP_FORCE: f32 = 16.0;
const JUMP_BUFFER_DISTANCE: f32 = 1.5;
const GRAVITY: f32 = 64.0;
const PLAYER_PUSH_FORCE: f32 = 28.0;
// const TERMINAL_VELOCITY: f32 = TODO

#[derive(Default)]
pub struct PlayerForces {
    pub movement: Vec3,
    pub external: Vec3,
    pub gravity: Vec3,
}

#[derive(Default, Component)]
pub struct PlayerMotion {
    pub grounded: bool,
    pub ground_normal: Option<Vec3>,
    pub ground_distance: Option<f32>,
    pub landed_time: f64,
    pub no_gravity_this_frame: bool,
    pub forces: PlayerForces,
}

#[derive(Resource, Default)]
pub struct PlayerInput {
    /// Commanded movement direction, local XZ plane.
    pub direction: Vec2,
    pub sprint: bool,

    #[cfg(feature = "crouch")]
    pub crouch: bool,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct PlayerActionBuffer(pub Vec<PlayerAction>);
impl PlayerActionBuffer {
    pub fn buffer(&mut self, action: PlayerAction) {
        if !self.0.contains(&action) {
            self.0.push(action);
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum PlayerAction {
    Jump,

    #[cfg(feature = "crouch")]
    Crouch(bool),
}

pub struct PlayerMotionPlugin;

impl Plugin for PlayerMotionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerInput>();
        app.init_resource::<PlayerActionBuffer>();
        app.add_systems(
            Update,
            (process_input, perform_actions, snap_to_ground, motion).chain(),
        );
    }
}

fn process_input(
    mut input: ResMut<PlayerInput>,
    mut actions: ResMut<PlayerActionBuffer>,
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
                    actions.buffer(PlayerAction::Jump);
                }
            }
        }
    };

    #[cfg(feature = "crouch")]
    if let Some(crouch) = &binds.crouch {
        if crouch.pressed(&keyboard, &mouse) {
            if !input.crouch {
                actions.buffer(PlayerAction::Crouch(true));
            }
        } else if input.crouch {
            actions.buffer(PlayerAction::Crouch(false));
        }
    }

    if let Some(sprint) = &binds.sprint {
        if sprint.pressed(&keyboard, &mouse) {
            input.sprint = true;
        }
    }
}

fn perform_actions(
    #[allow(unused)] mut input: ResMut<PlayerInput>,
    mut actions: ResMut<PlayerActionBuffer>,
    state: Option<Single<&mut PlayerMotion>>,
) {
    let Some(mut state) = state else {
        return;
    };

    actions.retain(|action| {
        let mut consumed = false;
        let mut consume = || consumed = true;

        match action {
            PlayerAction::Jump => {
                if state.grounded {
                    state.forces.gravity.y += JUMP_FORCE;
                    consume();
                }
            }

            #[cfg(feature = "crouch")]
            PlayerAction::Crouch(crouch) => {
                input.crouch = *crouch;
                consume();
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

    if state.forces.gravity.y <= 0.0 {
        transform.translation.y -= hit.distance - SKIN;
    }
    state.forces.gravity.y = state.forces.gravity.y.max(0.0);

    if !prev_grounded {
        state.landed_time = time.elapsed_secs_f64();
    }
}

fn motion(
    mut commands: Commands,
    centers: Query<(&Position, &ComputedCenterOfMass)>,
    time: Res<Time>,
    input: Res<PlayerInput>,
    spatial_query: SpatialQuery,
    player: Option<Single<(Entity, &mut Transform, &Section, &mut PlayerMotion)>>,
    camera: Option<Single<&GlobalTransform, With<PlayerCamera>>>,
    sensors: Query<Entity, With<Sensor>>,
) {
    let Some(player) = player else {
        return;
    };
    let Some(camera) = camera else {
        return;
    };

    let (entity, mut transform, section, mut state) = player.into_inner();
    let mut filter_entities: Vec<Entity> = sensors.iter().collect();
    filter_entities.push(entity);
    let filter = SpatialQueryFilter::from_excluded_entities(filter_entities);

    let mut collide_and_slide = |velocity: &mut Vec3| {
        collide_and_slide(
            &mut commands,
            &centers,
            section,
            velocity,
            &mut transform.translation,
            &spatial_query,
            &filter,
            &time,
        );
    };

    // External force
    {
        state.forces.external *= 1.0 - time.delta_secs() * 4.0;
        collide_and_slide(&mut state.forces.external);
    }

    // Movement
    {
        let mut wishdir = Vec3::new(input.direction.x, 0.0, input.direction.y);
        let yaw = camera.rotation().to_euler(EulerRot::YXZ).0;
        let rotation = Transform::from_rotation(Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.0));
        wishdir = rotation.transform_point(wishdir);
        if state.grounded {
            ground_move(
                wishdir,
                state.landed_time,
                &mut state.forces.movement,
                &time,
            );
        } else {
            air_move(wishdir, &mut state.forces.movement, &time);
        }
        collide_and_slide(&mut state.forces.movement);
    };

    // Gravity
    'gravity: {
        if state.no_gravity_this_frame {
            state.no_gravity_this_frame = false;
            break 'gravity;
        }
        let mut gravity = Vec3::NEG_Y * GRAVITY * time.delta_secs();
        if state.grounded {
            gravity *= 0.01;
        }
        state.forces.gravity += gravity;
        collide_and_slide(&mut state.forces.gravity)
    };

    // Just in case
    depenetrate(&spatial_query, &filter, &section.collider(), &mut transform);
}

//
// Utility
//

fn collide_and_slide(
    commands: &mut Commands,
    centers: &Query<(&Position, &ComputedCenterOfMass)>,
    section: &Section,
    velocity: &mut Vec3,
    position: &mut Vec3,
    spatial_query: &SpatialQuery,
    filter: &SpatialQueryFilter,
    time: &Res<Time>,
) {
    let mut forces = Vec::<(Entity, Vec3, ExternalForce)>::new();

    for _ in 0..MAX_BOUNCES {
        let timescaled_velocity = *velocity * time.delta_secs();

        let Ok((direction, distance)) = Dir3::new_and_length(timescaled_velocity) else {
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
                ignore_origin_penetration: false,
            },
            filter,
        );
        let Some(hit) = shapecast else {
            *position += timescaled_velocity;
            break;
        };

        let ratio = hit.distance / timescaled_velocity.length();
        let rejection = *velocity * hit.normal1 * hit.normal1;

        *position += timescaled_velocity * ratio + hit.normal1 * SKIN;
        *velocity -= rejection;

        if let Some((_, center, force)) = forces
            .iter_mut()
            .find(|(entity, _, _)| *entity == hit.entity)
        {
            force.apply_force_at_point(rejection * PLAYER_PUSH_FORCE, hit.point1, *center);
        } else if let Ok((position, local_center)) = centers.get(hit.entity) {
            let center = position.0 + local_center.0;
            let mut force = ExternalForce::default().with_persistence(false);
            force.apply_force_at_point(rejection * PLAYER_PUSH_FORCE, hit.point1, center);
            forces.push((hit.entity, center, force));
        }
    }

    for (entity, _, force) in forces {
        commands.entity(entity).insert(force);
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
