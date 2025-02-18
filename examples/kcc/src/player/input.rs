use bevy::prelude::*;

use super::{
    config::PlayerMotionConfig,
    motion::{PlayerAction, PlayerActionBuffer, PlayerInput},
    PlayerKeybinds, PlayerMotion,
};

pub struct PlayerInputPlugin;

impl Plugin for PlayerInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (process_input, perform_actions).chain());
    }
}

pub fn process_input(
    mut input: ResMut<PlayerInput>,
    mut actions: ResMut<PlayerActionBuffer>,
    binds: Res<PlayerKeybinds>,
    keyboard: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    #[allow(unused)] motion_config: Res<PlayerMotionConfig>,
    #[allow(unused)] state: Option<Single<&PlayerMotion>>,
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

    #[cfg(feature = "jump")]
    if let Some(jump) = &binds.jump {
        if let Some(state) = state {
            if let Some(ground_distance) = state.ground_distance {
                if jump.just_pressed(&keyboard, &mouse)
                    && ground_distance <= motion_config.jump_buffer_distance
                {
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

pub fn perform_actions(
    mut actions: ResMut<PlayerActionBuffer>,
    #[allow(unused)] mut input: ResMut<PlayerInput>,
    #[allow(unused)] motion_config: Res<PlayerMotionConfig>,
    #[allow(unused)] state: Option<Single<&mut PlayerMotion>>,
) {
    #[cfg(feature = "jump")]
    let Some(mut state) = state
    else {
        return;
    };

    actions.retain(|action| {
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
        };

        !consumed
    });
}
