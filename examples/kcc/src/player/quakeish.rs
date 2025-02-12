// References
// - https://github.com/id-Software/Quake/blob/master/QW/client/pmove.c
// - https://adrianb.io/2015/02/14/bunnyhop.html

use bevy::prelude::{Vec3, *};

// Ratios based on Quake/QW/server/sv_phys.c
// Not sure how 1:1 this is with Quake.
const FRICTION: f32 = 6.0;
const FRICTION_DELAY_SECS: f64 = 1.0 / 20.0;
const QUAKE_UNITS_PER_METER: f32 = 16.0;
const GROUND_ACCELERATE: f32 = 10.0 * QUAKE_UNITS_PER_METER;
const AIR_ACCELERATE: f32 = 0.7 * QUAKE_UNITS_PER_METER;
const MAX_VELOCITY_GROUND: f32 = 320.0 / QUAKE_UNITS_PER_METER;
const MAX_VELOCITY_AIR: f32 = 320.0 / QUAKE_UNITS_PER_METER;

pub fn accelerate(
    direction: Vec3,
    curr_velocity: Vec3,
    acceleration: f32,
    max_velocity: f32,
    time: &Res<Time>,
) -> Vec3 {
    let projected = curr_velocity.dot(direction);
    let mut acceleration = acceleration * time.delta_secs();

    if projected + acceleration > max_velocity {
        acceleration = max_velocity - projected;
    }

    curr_velocity + direction * acceleration
}

pub fn ground_move(direction: Vec3, landed_time: f64, curr_velocity: &mut Vec3, time: &Res<Time>) {
    let speed = curr_velocity.length();

    if time.elapsed_secs_f64() - landed_time >= FRICTION_DELAY_SECS && speed != 0.0 {
        let drop = speed * FRICTION * time.delta_secs();
        *curr_velocity *= f32::max(speed - drop, 0.0) / speed;
    }

    *curr_velocity = accelerate(
        direction,
        *curr_velocity,
        GROUND_ACCELERATE,
        MAX_VELOCITY_GROUND,
        time,
    );
}
pub fn air_move(direction: Vec3, curr_velocity: &mut Vec3, time: &Res<Time>) {
    *curr_velocity = accelerate(
        direction,
        *curr_velocity,
        AIR_ACCELERATE,
        MAX_VELOCITY_AIR,
        time,
    );
}
