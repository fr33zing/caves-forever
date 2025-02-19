// References
// - https://github.com/id-Software/Quake/blob/master/QW/client/pmove.c
// - https://adrianb.io/2015/02/14/bunnyhop.html

use bevy::prelude::{Vec3, *};

use super::config::PlayerMotionConfig;

pub fn accelerate(
    direction: Dir3,
    curr_velocity: Vec3,
    acceleration: f32,
    max_velocity: f32,
    time: &Res<Time>,
) -> Vec3 {
    let projected = curr_velocity.dot(*direction);
    let mut acceleration = acceleration * time.delta_secs();

    if projected + acceleration > max_velocity {
        acceleration = max_velocity - projected;
    }

    curr_velocity + direction * acceleration
}

pub fn ground_move(
    direction: Dir3,
    landed_time: f64,
    curr_velocity: &mut Vec3,
    time: &Res<Time>,
    speed_mod: f32,
    motion_config: &Res<PlayerMotionConfig>,
) {
    let speed = curr_velocity.length();

    if time.elapsed_secs_f64() - landed_time >= motion_config.friction_delay_secs && speed != 0.0 {
        let drop = speed * motion_config.friction * time.delta_secs();
        *curr_velocity *= f32::max(speed - drop, 0.0) / speed;
    }

    *curr_velocity = accelerate(
        direction,
        *curr_velocity,
        motion_config.ground_accelerate * speed_mod,
        motion_config.max_velocity_ground * speed_mod,
        time,
    );
}
pub fn air_move(
    direction: Dir3,
    curr_velocity: &mut Vec3,
    time: &Res<Time>,
    speed_mod: f32,
    motion_config: &Res<PlayerMotionConfig>,
) {
    *curr_velocity = accelerate(
        direction,
        *curr_velocity,
        motion_config.air_accelerate * speed_mod,
        motion_config.max_velocity_air * speed_mod,
        time,
    );
}
