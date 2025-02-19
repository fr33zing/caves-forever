use bevy::prelude::*;

use crate::player::{
    config::{PlayerActionsConfig, SlideActionConfig},
    input::PlayerInput,
    motion::PlayerForces,
    PlayerMotion,
};

pub struct PlayerSlidePlugin;

impl Plugin for PlayerSlidePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, sliding);
    }
}

pub fn can_stop_sliding(slide_config: &SlideActionConfig, forces: &PlayerForces) -> bool {
    (forces.external + forces.movement).length() < slide_config.stop_sliding_velocity
}

fn sliding(
    input: Res<PlayerInput>,
    time: Res<Time>,
    actions_config: Res<PlayerActionsConfig>,
    state: Option<Single<&mut PlayerMotion>>,
) {
    if !input.slide {
        return;
    }
    let Some(mut state) = state else {
        return;
    };
    let Some(ground_normal) = state.ground_normal else {
        return;
    };
    let Some(slide_config) = &actions_config.slide else {
        return;
    };

    let (min_slope, max_slope) = (
        slide_config.min_acceleration_slope_degrees,
        slide_config.max_acceleration_slope_degrees,
    );

    let slope_degrees = (180.0 - ground_normal.dot(Vec3::Y) * 180.0).clamp(0.0, max_slope);

    if slope_degrees < min_slope {
        return;
    }

    let ratio = (slope_degrees - min_slope) / (max_slope - min_slope);
    let acceleration = ratio * slide_config.max_slope_acceleration * time.delta_secs();
    let direction = Vec3::NEG_Y.reject_from(ground_normal);

    state.forces.external += acceleration * direction;
}
