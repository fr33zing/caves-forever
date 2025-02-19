use bevy::prelude::*;

use crate::player::{
    config::PlayerActionsConfig, input::PlayerInput, Player, PlayerConfig, PlayerMotion, Section,
};

const CROUCH_EPSILON: f32 = 0.0001;

pub struct PlayerCrouchPlugin;

impl Plugin for PlayerCrouchPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, crouch);
    }
}

fn crouch(
    mut commands: Commands,
    actions_config: Res<PlayerActionsConfig>,
    player: Option<Single<(Entity, &mut Section, &mut Transform, &PlayerMotion), With<Player>>>,
    time: Res<Time>,
    input: Res<PlayerInput>,
    config: Res<PlayerConfig>,

    // TEMP
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(crouch_config) = &actions_config.crouch else {
        return;
    };
    let Some(player) = player else {
        return;
    };
    let (entity, mut section, mut transform, state) = player.into_inner();

    let target_height = if input.crouch {
        config.height / 2.0
    } else {
        config.height
    };

    let t = (crouch_config.transition_speed * time.delta_secs()).clamp(0.0, 1.0);
    let height = section.height.lerp(target_height, t);
    let mut diff = height - section.height;

    if diff.abs() > CROUCH_EPSILON {
        section.height = height;
    } else {
        diff = target_height - section.height;
        section.height = target_height;
    };

    let mut commands = commands.entity(entity);

    if diff != 0.0 {
        if let Some(crouch) = &actions_config.crouch {
            if crouch.crouchjump_additional_clearance {
                if state.grounded {
                    transform.translation.y += section.offset;
                    section.offset = 0.0;
                } else {
                    section.offset -= diff;
                }
            }
        }

        commands.insert(section.collider());
        commands.insert(Mesh3d(meshes.add(section.mesh())));
    }
}
