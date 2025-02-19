use bevy::prelude::*;

use super::{
    config::PlayerActionsConfig,
    input::{process_input, PlayerInput},
    PlayerMotion,
};

mod crouch;
use crouch::PlayerCrouchPlugin;

#[derive(PartialEq)]
pub enum PlayerAction {
    Jump,
    Crouch(bool),
}

pub struct BufferedPlayerAction {
    pub time: f64,
    pub action: PlayerAction,
}

#[derive(Resource, Default, Deref, DerefMut)]
pub struct PlayerActionBuffer(pub Vec<BufferedPlayerAction>);

impl PlayerActionBuffer {
    pub fn buffer(&mut self, action: PlayerAction, now: f64) {
        self.0.retain(|x| x.action != action);
        self.0.push(BufferedPlayerAction { action, time: now });
    }
}

pub struct PlayerActionsPlugin;

impl Plugin for PlayerActionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PlayerCrouchPlugin);

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
    mut actions: ResMut<PlayerActionBuffer>,
    mut input: ResMut<PlayerInput>,
) {
    let Some(mut state) = state else {
        return;
    };

    let now = time.elapsed_secs_f64();
    actions.retain(|BufferedPlayerAction { action, time }| {
        let elapsed = now - time;
        let expired = if let Some(expiry) = actions_config.expiry_for(action) {
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
            PlayerAction::Jump => {
                let Some(jump) = &actions_config.jump else {
                    return false;
                };

                if state.grounded {
                    state.forces.gravity.y += jump.force;
                    consume();
                }
            }

            PlayerAction::Crouch(crouch) => {
                if actions_config.crouch.is_none() {
                    return false;
                };

                input.crouch = *crouch;
                consume();
            }
        };

        !consumed
    });
}
