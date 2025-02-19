use bevy::prelude::*;

#[cfg(feature = "input")]
use super::{config::PlayerKeybinds, PlayerInputConfig, PlayerWalkModMode};

use super::{actions::PlayerActionBuffer, config::PlayerActionsConfig, PlayerMotion};

#[derive(Resource, Default)]
pub struct PlayerYaw(pub f32);

#[derive(Resource, Default)]
pub struct PlayerInput {
    /// Commanded movement direction, local XZ plane.
    pub direction: Vec2,
    pub walk_mod: bool,
    pub crouch: bool,
}

pub struct PlayerInputPlugin;

impl Plugin for PlayerInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerInput>();
        app.init_resource::<PlayerInputConfig>();
        app.init_resource::<PlayerActionsConfig>();
        app.init_resource::<PlayerActionBuffer>();
        app.init_resource::<PlayerYaw>();

        #[cfg(feature = "input")]
        app.add_systems(Update, process_input);
    }
}

#[cfg(feature = "input")]
pub fn process_input(
    mut input: ResMut<PlayerInput>,
    mut actions: ResMut<PlayerActionBuffer>,
    time: Res<Time>,
    input_config: Res<PlayerInputConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    actions_config: Res<PlayerActionsConfig>,
    state: Option<Single<&PlayerMotion>>,
) {
    use super::actions::PlayerAction;

    let now = time.elapsed_secs_f64();

    input.direction = Vec2::ZERO;

    if let Some(forward) = &input_config.binds.forward {
        if forward.pressed(&keyboard, &mouse) {
            input.direction += Vec2::NEG_Y;
        }
    }
    if let Some(backward) = &input_config.binds.backward {
        if backward.pressed(&keyboard, &mouse) {
            input.direction += Vec2::Y;
        }
    }
    if let Some(left) = &input_config.binds.left {
        if left.pressed(&keyboard, &mouse) {
            input.direction += Vec2::NEG_X;
        }
    }
    if let Some(right) = &input_config.binds.right {
        if right.pressed(&keyboard, &mouse) {
            input.direction += Vec2::X;
        }
    }

    if input.direction.length() > 0.0 {
        input.direction = input.direction.normalize();
    }

    if let (Some(jump_bind), Some(jump_config)) = (&input_config.binds.jump, &actions_config.jump) {
        if let Some(state) = state {
            if let Some(ground_distance) = state.ground_distance {
                if jump_bind.just_pressed(&keyboard, &mouse)
                    && ground_distance <= jump_config.buffer_distance
                {
                    actions.buffer(PlayerAction::Jump, now);
                }
            }
        }
    };

    if let (Some(crouch_bind), Some(_)) = (&input_config.binds.crouch, &actions_config.crouch) {
        if crouch_bind.pressed(&keyboard, &mouse) {
            if !input.crouch {
                actions.buffer(PlayerAction::Crouch(true), now);
            }
        } else if input.crouch {
            actions.buffer(PlayerAction::Crouch(false), now);
        }
    }

    if let Some(walk_mod) = &input_config.binds.walk_mod {
        match input_config.walk_mod_mode {
            PlayerWalkModMode::Hold => {
                input.walk_mod = walk_mod.pressed(&keyboard, &mouse);
            }
            PlayerWalkModMode::Toggle => {
                if walk_mod.just_pressed(&keyboard, &mouse) {
                    input.walk_mod = !input.walk_mod;
                }
            }
            _ => {
                let moving = PlayerKeybinds::any_pressed(
                    [
                        &input_config.binds.forward,
                        &input_config.binds.backward,
                        &input_config.binds.left,
                        &input_config.binds.right,
                    ],
                    &keyboard,
                    &mouse,
                );

                match input_config.walk_mod_mode {
                    PlayerWalkModMode::ToggleHybrid => {
                        input.walk_mod = if walk_mod.just_pressed(&keyboard, &mouse) {
                            !input.walk_mod
                        } else if input.walk_mod {
                            moving
                        } else {
                            walk_mod.just_pressed(&keyboard, &mouse)
                        };
                    }
                    PlayerWalkModMode::Hybrid => {
                        input.walk_mod = if input.walk_mod {
                            moving
                        } else {
                            walk_mod.just_pressed(&keyboard, &mouse)
                        };
                    }
                    _ => unreachable!(),
                }
            }
        }
    }
}
