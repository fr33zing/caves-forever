// References
// - https://github.com/id-Software/Quake/blob/master/QW/client/pmove.c
// - https://adrianb.io/2015/02/14/bunnyhop.html

use bevy::prelude::{Vec3, *};

const FRICTION: f32 = 5.0;
const GROUND_ACCELERATE: f32 = 128.0;
const AIR_ACCELERATE: f32 = 32.0;
const MAX_VELOCITY_GROUND: f32 = 20000.0;
const MAX_VELOCITY_AIR: f32 = 200.0;

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

pub fn ground_move(direction: Vec3, curr_velocity: &mut Vec3, time: &Res<Time>) {
    let speed = curr_velocity.length();

    if speed != 0.0 {
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
pub fn air_move(direction: Vec3, curr_velocity: &mut Vec3, time: &Res<Time>) -> Vec3 {
    accelerate(
        direction,
        *curr_velocity,
        AIR_ACCELERATE,
        MAX_VELOCITY_AIR,
        time,
    )
}
