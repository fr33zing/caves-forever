// References:
// - https://www.peroxide.dk/papers/collision/collision.pdf
// - https://github.com/Jondolf/avian/blob/45d9f8fa16c28530e77d1c96e7d600cbf2b46fad/crates/avian3d/examples/collide_and_slide_3d/plugin.rs
// - https://github.com/nicholas-maltbie/OpenKCC/blob/a1a30ed7f7722ea82a1df6bd01849e0bfde6abf4/Assets/Samples/SimplifiedDemoKCC/Scripts/SimplifiedKCC.cs
// - https://github.com/Desine-Unity/collide-and-slide/blob/main/Runtime/CollideAndSlide.cs

// TODO add an expiration time to buffered actions

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{
    actions,
    config::{PlayerActionsConfig, PlayerMotionConfig},
    input::{self, PlayerInput, PlayerYaw},
    quakeish::{air_move, ground_move},
    utility::{running, wish_dir},
    PlayerInputConfig, Section,
};

#[derive(Default)]
pub struct PlayerForces {
    pub movement: Vec3,
    pub external: Vec3,
    pub gravity: Vec3,
}
impl PlayerForces {
    pub fn sum(&self) -> Vec3 {
        self.movement + self.external + self.gravity
    }
}

#[derive(Component, Default)]
pub struct PlayerMotion {
    pub grounded: bool,
    pub ground_normal: Option<Vec3>,
    pub ground_distance: Option<f32>,
    pub landed_time: f64,
    pub no_gravity_this_frame: bool,
    pub forces: PlayerForces,
}

pub struct PlayerMotionPlugin;

impl Plugin for PlayerMotionPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerMotionConfig>();

        #[cfg(feature = "input")]
        app.add_systems(
            Update,
            (snap_to_ground, motion)
                .after(input::process_input)
                .after(actions::perform_actions)
                .chain(),
        );
        #[cfg(not(feature = "input"))]
        app.add_systems(
            Update,
            (snap_to_ground, motion)
                .after(actions::perform_actions)
                .chain(),
        );
    }
}

fn snap_to_ground(
    time: Res<Time>,
    spatial_query: SpatialQuery,
    motion_config: Res<PlayerMotionConfig>,
    actions_config: Res<PlayerActionsConfig>,
    input: Res<PlayerInput>,
    player: Option<Single<(Entity, &mut Transform, &Section, &mut PlayerMotion)>>,
) {
    let Some(player) = player else {
        return;
    };

    let (entity, mut transform, section, mut state) = player.into_inner();

    let distance = if let Some(jump) = &actions_config.jump {
        jump.buffer_distance
    } else {
        motion_config.snap_to_ground_distance
    };

    let shapecast = spatial_query.cast_shape(
        &section.collider(),
        transform.translation,
        default(),
        Dir3::NEG_Y,
        &ShapeCastConfig::from_max_distance(distance),
        &SpatialQueryFilter::from_excluded_entities(vec![entity]),
    );
    let Some(hit) = shapecast else {
        state.grounded = false;
        state.ground_distance = None;
        return;
    };

    state.ground_distance = Some(hit.distance);

    if hit.distance > motion_config.snap_to_ground_distance {
        state.grounded = false;
        return;
    }

    let prev_grounded = state.grounded;

    let angle = hit.normal1.angle_between(Vec3::Y);
    state.grounded = angle < motion_config.max_slope_degrees.to_radians();
    state.ground_normal = Some(hit.normal1);

    if !state.grounded {
        return;
    }

    if state.forces.gravity.y <= 0.0 {
        transform.translation.y -= hit.distance - motion_config.skin;
    }

    if !input.slide {
        state.forces.gravity.y = state.forces.gravity.y.max(0.0);
    }

    if !prev_grounded {
        state.landed_time = time.elapsed_secs_f64();
    }
}

fn motion(
    mut commands: Commands,
    centers: Query<(&Position, &ComputedCenterOfMass)>,
    time: Res<Time>,
    input: Res<PlayerInput>,
    input_config: Res<PlayerInputConfig>,
    motion_config: Res<PlayerMotionConfig>,
    spatial_query: SpatialQuery,
    player: Option<Single<(Entity, &mut Transform, &Section, &mut PlayerMotion)>>,
    sensors: Query<Entity, With<Sensor>>,
    yaw: Res<PlayerYaw>,
) {
    let Some(player) = player else {
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
            motion_config.collide_and_slide_bounces,
            motion_config.skin,
            motion_config.push_force,
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
        let wish_dir = wish_dir(&yaw, &input);
        let speed_mod = match running(&input, &input_config) {
            false => 1.0,
            true => motion_config.run_speed_mod,
        };

        if state.grounded {
            ground_move(
                wish_dir,
                state.landed_time,
                &mut state.forces.movement,
                &time,
                speed_mod,
                &motion_config,
            );
        } else {
            air_move(
                wish_dir,
                &mut state.forces.movement,
                &time,
                speed_mod,
                &motion_config,
            );
        }
        collide_and_slide(&mut state.forces.movement);
    };

    // Gravity
    'gravity: {
        if state.no_gravity_this_frame {
            state.no_gravity_this_frame = false;
            break 'gravity;
        }
        let mut gravity = Vec3::NEG_Y * motion_config.gravity * time.delta_secs();
        if state.grounded {
            gravity *= 0.01;
        }
        state.forces.gravity += gravity;
        collide_and_slide(&mut state.forces.gravity)
    };

    // Just in case
    depenetrate(
        &spatial_query,
        &filter,
        motion_config.skin,
        &section.collider(),
        &mut transform,
    );
}

//
// Utility
//

fn collide_and_slide(
    commands: &mut Commands,
    centers: &Query<(&Position, &ComputedCenterOfMass)>,
    max_bounces: u8,
    skin: f32,
    push_force: f32,
    section: &Section,
    velocity: &mut Vec3,
    position: &mut Vec3,
    spatial_query: &SpatialQuery,
    filter: &SpatialQueryFilter,
    time: &Res<Time>,
) {
    let mut forces = Vec::<(Entity, Vec3, ExternalForce)>::new();

    for _ in 0..max_bounces {
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

        *position += timescaled_velocity * ratio + hit.normal1 * skin;
        *velocity -= rejection;

        if let Some((_, center, force)) = forces
            .iter_mut()
            .find(|(entity, _, _)| *entity == hit.entity)
        {
            force.apply_force_at_point(rejection * push_force, hit.point1, *center);
        } else if let Ok((position, local_center)) = centers.get(hit.entity) {
            let center = position.0 + local_center.0;
            let mut force = ExternalForce::default().with_persistence(false);
            force.apply_force_at_point(rejection * push_force, hit.point1, center);
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
    skin: f32,
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
        transform.translation += hit.normal1 * (hit.distance + skin);
    }
}
