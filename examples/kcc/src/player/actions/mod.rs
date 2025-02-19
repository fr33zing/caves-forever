use bevy::prelude::*;
use slide::PlayerSlidePlugin;

use super::{
    config::PlayerActionsConfig,
    input::{process_input, PlayerInput, PlayerYaw},
    utility::wish_dir,
    PlayerMotion,
};

mod slide;

mod crouch;
pub use crouch::can_stand;
use crouch::PlayerCrouchPlugin;

#[derive(PartialEq)]
pub enum PlayerAction {
    Jump,
    Crouch(bool),
    Slide,
}

pub enum ActionExpiry {
    Instant,

    /// .0 = buffered time, not expiration time
    Timed(f64),
}

pub struct BufferedPlayerAction {
    pub expiry: ActionExpiry,
    pub action: PlayerAction,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct PlayerActionBuffer(pub Vec<BufferedPlayerAction>);

impl PlayerActionBuffer {
    pub fn instant(&mut self, action: PlayerAction) {
        self.0.retain(|b| b.action != action);
        self.0.push(BufferedPlayerAction {
            action,
            expiry: ActionExpiry::Instant,
        });
    }

    pub fn buffer(&mut self, action: PlayerAction, now: f64) {
        self.0.retain(|b| b.action != action);
        self.0.push(BufferedPlayerAction {
            action,
            expiry: ActionExpiry::Timed(now),
        });
    }
}

pub struct PlayerActionsPlugin;

impl Plugin for PlayerActionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((PlayerCrouchPlugin, PlayerSlidePlugin));

        #[cfg(feature = "input")]
        app.add_systems(Update, perform_actions.after(process_input));

        #[cfg(not(feature = "input"))]
        app.add_systems(Update, perform_actions);
    }
}

pub fn perform_actions(
    time: Res<Time>,
    state: Option<Single<&mut PlayerMotion>>,
    actions_config: Res<PlayerActionsConfig>,
    yaw: Res<PlayerYaw>,
    mut actions: ResMut<PlayerActionBuffer>,
    mut input: ResMut<PlayerInput>,
) {
    let Some(mut state) = state else {
        return;
    };

    // Clear expired "Timed" actions
    let now = time.elapsed_secs_f64();
    actions.retain(|BufferedPlayerAction { action, expiry }| {
        let ActionExpiry::Timed(buffered_time) = expiry else {
            return true;
        };

        let elapsed = now - buffered_time;
        let expired = if let Some(expiry) = actions_config.expiry_for(action) {
            elapsed >= expiry
        } else {
            false
        };

        !expired
    });

    // Consume actions
    actions.retain(|BufferedPlayerAction { action, .. }| {
        let mut consumed = false;
        let mut consume = || consumed = true;

        match action {
            PlayerAction::Jump => {
                let Some(jump_config) = &actions_config.jump else {
                    return false;
                };

                if state.grounded {
                    state.forces.gravity.y += jump_config.force;
                    consume();
                }
            }

            PlayerAction::Crouch(crouch) => {
                let Some(_) = &actions_config.crouch else {
                    return false;
                };

                if !crouch {
                    input.slide = false;
                }

                input.crouch = *crouch;
                consume();
            }

            PlayerAction::Slide => {
                let Some(slide_config) = &actions_config.slide else {
                    return false;
                };
                if !state.grounded {
                    return false;
                };
                let Some(ground_normal) = state.ground_normal else {
                    return false;
                };

                let wish_dir = wish_dir(&yaw, &input);
                let slide_force = slide_config.force * wish_dir.reject_from(ground_normal);
                state.forces.external += slide_force;

                input.slide = true;
                consume();
            }
        };

        !consumed
    });

    // Clear all unconsumed "Instant" actions
    actions.retain(|BufferedPlayerAction { expiry, .. }| !matches!(expiry, ActionExpiry::Instant));
}
