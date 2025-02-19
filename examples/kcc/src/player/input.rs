use bevy::prelude::*;

#[cfg(feature = "input")]
use super::{
    actions::PlayerAction, config::PlayerKeybinds, utility::running, PlayerInputConfig,
    PlayerWalkModMode,
};

use super::{actions::PlayerActionBuffer, config::PlayerActionsConfig, PlayerMotion};

#[derive(Resource, Default)]
pub struct PlayerYaw(pub f32);

#[derive(Resource, Default)]
pub struct PlayerInput {
    /// Commanded movement direction, local XZ plane.
    pub direction: Vec2,
    pub walk_mod: bool,
    pub crouch: bool,
    pub slide: bool,
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
    actions_config: Res<PlayerActionsConfig>,
    input_config: Res<PlayerInputConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    state: Option<Single<&PlayerMotion>>,
) {
    use super::actions::can_stand;

    let Some(state) = state else {
        return;
    };

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
        if let Some(ground_distance) = state.ground_distance {
            if jump_bind.just_pressed(&keyboard, &mouse)
                && ground_distance <= jump_config.buffer_distance
            {
                if jump_config.bufferable {
                    actions.buffer(PlayerAction::Jump, now);
                } else {
                    actions.instant(PlayerAction::Jump);
                }
            }
        }
    };

    if let (Some(crouch_bind), Some(crouch_config)) =
        (&input_config.binds.crouch, &actions_config.crouch)
    {
        if crouch_bind.pressed(&keyboard, &mouse) {
            if !input.crouch {
                if crouch_config.slide_if_running && !input.crouch && running(&input, &input_config)
                {
                    actions.instant(PlayerAction::Slide);
                }

                actions.instant(PlayerAction::Crouch(true));
            }
        } else if input.crouch && can_stand(&input, &actions_config, &state.forces) {
            actions.instant(PlayerAction::Crouch(false));
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
