use bevy::prelude::*;

#[cfg(feature = "input")]
use super::PlayerInputConfig;

#[cfg(feature = "jump")]
use super::{
    config::{PlayerBufferedActionsConfig, PlayerMotionConfig},
    PlayerMotion,
};

#[derive(Resource, Default)]
pub struct PlayerYaw(pub f32);

#[derive(Resource, Default)]
pub struct PlayerInput {
    /// Commanded movement direction, local XZ plane.
    pub direction: Vec2,
    pub walk_mod: bool,

    #[cfg(feature = "crouch")]
    pub crouch: bool,
}

#[cfg(feature = "actions")]
#[derive(Resource, Default, Deref, DerefMut)]
pub struct PlayerActionBuffer(pub Vec<BufferedPlayerAction>);

#[cfg(feature = "actions")]
impl PlayerActionBuffer {
    pub fn buffer(&mut self, action: PlayerAction, now: f64) {
        self.0.retain(|x| x.action != action);
        self.0.push(BufferedPlayerAction { action, time: now });
    }
}

#[cfg(feature = "actions")]
pub struct BufferedPlayerAction {
    pub time: f64,
    pub action: PlayerAction,
}

#[cfg(feature = "actions")]
#[derive(PartialEq)]
pub enum PlayerAction {
    #[cfg(feature = "jump")]
    Jump,

    #[cfg(feature = "crouch")]
    Crouch(bool),
}

pub struct PlayerInputPlugin;

impl Plugin for PlayerInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerInput>();
        app.init_resource::<PlayerInputConfig>();
        app.init_resource::<PlayerYaw>();

        #[cfg(feature = "actions")]
        app.init_resource::<PlayerActionBuffer>();

        #[cfg(feature = "jump")]
        app.init_resource::<PlayerBufferedActionsConfig>();

        #[cfg(feature = "input")]
        app.add_systems(Update, (process_input, perform_actions).chain());
        #[cfg(all(not(feature = "input"), feature = "actions"))]
        app.add_systems(Update, perform_actions);
    }
}

#[cfg(feature = "input")]
pub fn process_input(
    mut input: ResMut<PlayerInput>,
    #[allow(unused_mut, unused)] mut actions: ResMut<PlayerActionBuffer>,
    #[cfg(any(feature = "jump", feature = "crouch"))] time: Res<Time>,
    input_config: Res<PlayerInputConfig>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    #[cfg(feature = "jump")] buffer_config: Res<PlayerBufferedActionsConfig>,
    #[cfg(feature = "jump")] state: Option<Single<&PlayerMotion>>,
) {
    use crate::player::config::PlayerKeybinds;

    use super::config::PlayerWalkModMode;

    #[cfg(any(feature = "jump", feature = "crouch"))]
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

    #[cfg(feature = "jump")]
    if let Some(jump) = &input_config.binds.jump {
        if let Some(state) = state {
            if let Some(ground_distance) = state.ground_distance {
                if jump.just_pressed(&keyboard, &mouse)
                    && ground_distance <= buffer_config.jump_buffer_distance
                {
                    actions.buffer(PlayerAction::Jump, now);
                }
            }
        }
    };

    #[cfg(feature = "crouch")]
    if let Some(crouch) = &input_config.binds.crouch {
        if crouch.pressed(&keyboard, &mouse) {
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

#[cfg(any(feature = "jump", feature = "crouch"))]
pub fn perform_actions(
    time: Res<Time>,
    buffer_config: Res<PlayerBufferedActionsConfig>,
    mut actions: ResMut<PlayerActionBuffer>,
    #[cfg(feature = "jump")] motion_config: Res<PlayerMotionConfig>,
    #[cfg(feature = "jump")] state: Option<Single<&mut PlayerMotion>>,
    #[cfg(feature = "crouch")] mut input: ResMut<PlayerInput>,
) {
    #[cfg(feature = "jump")]
    let Some(mut state) = state
    else {
        return;
    };

    let now = time.elapsed_secs_f64();
    actions.retain(|BufferedPlayerAction { action, time }| {
        let elapsed = now - time;
        let expired = if let Some(expiry) = buffer_config.expiry_for(action) {
            elapsed >= expiry
        } else {
            false
        };

        !expired
    });

    actions.retain(|BufferedPlayerAction { action, .. }| {
        let mut consumed = false;
        let mut consume = || consumed = true;

        match action {
            #[cfg(feature = "jump")]
            PlayerAction::Jump => {
                if state.grounded {
                    state.forces.gravity.y += motion_config.jump_force;
                    consume();
                }
            }

            #[cfg(feature = "crouch")]
            PlayerAction::Crouch(crouch) => {
                input.crouch = *crouch;
                consume();
            }

            #[allow(unreachable_patterns)]
            _ => {}
        };

        !consumed
    });
}
