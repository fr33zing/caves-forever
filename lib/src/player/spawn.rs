use std::f32::consts::FRAC_PI_4;

use avian3d::prelude::{Collider, LockedAxes, RigidBody};
use bevy::{ecs::system::SystemState, prelude::*};
use bevy_tnua::{
    builtins::TnuaBuiltinCrouch,
    control_helpers::{TnuaCrouchEnforcer, TnuaSimpleAirActionsCounter},
    math::{Float, Vector3},
    prelude::{TnuaBuiltinJump, TnuaBuiltinWalk, TnuaController},
};
use bevy_tnua_avian3d::TnuaAvian3dSensorShape;
use rand::seq::SliceRandom;

use crate::worldgen::layout::{LayoutState, Spawnpoint};

use super::{
    camera::{Flashlight, PlayerCamera},
    controls::PlayerMotionConfig,
    ForwardFromCamera, IsPlayer, PLAYER_COLLIDER, PLAYER_FLOAT_HEIGHT_FROM_CENTER,
};

pub struct DespawnPlayerCommand;

#[derive(Default)]
pub struct SpawnPlayerCommand {
    /// If no position is provided, a random spawnpoint entity will be selected.
    pub position: Option<Vec3>,
}

impl Command for DespawnPlayerCommand {
    fn apply(self, world: &mut World) {
        let mut system_state: SystemState<(
            Commands,
            Option<Single<Entity, With<IsPlayer>>>,
            Option<Single<Entity, With<PlayerCamera>>>,
        )> = SystemState::new(world);
        let (mut commands, player, camera) = system_state.get_mut(world);

        if let Some(player) = player {
            commands.entity(*player).clear();
        };
        if let Some(camera) = camera {
            commands.entity(*camera).clear();
        };

        system_state.apply(world);
    }
}

impl Command for SpawnPlayerCommand {
    fn apply(self, world: &mut World) {
        let mut system_state: SystemState<(
            Commands,
            Option<ResMut<LayoutState>>,
            Query<&GlobalTransform, With<Spawnpoint>>,
        )> = SystemState::new(world);
        let (mut commands, layout_state, spawnpoints) = system_state.get_mut(world);

        let position = self.position.unwrap_or_else(|| {
            let spawnpoints = spawnpoints
                .iter()
                .map(|s| s.translation())
                .collect::<Vec<_>>();
            *spawnpoints
                .choose(&mut layout_state.unwrap().rng)
                .expect("no spawnpoints")
        });

        // Camera
        commands.spawn((
            PlayerCamera,
            Camera3d::default(),
            Camera {
                order: 2,
                ..default()
            },
            Projection::Perspective(PerspectiveProjection {
                fov: 45.0_f32.to_radians(),
                ..default()
            }),
            Flashlight(10_000_000.0),
            SpotLight {
                intensity: 10_000_000.0,
                color: Color::WHITE.into(),
                shadows_enabled: true,
                inner_angle: 0.35,
                outer_angle: 0.45,
                range: 4000.0,
                radius: 4000.0,
                ..default()
            },
        ));

        // Player
        let mut commands = commands.spawn(IsPlayer);
        commands.insert(Transform::from_translation(position));
        commands.insert(RigidBody::Dynamic);
        commands.insert(LockedAxes::new().lock_rotation_x().lock_rotation_z());
        commands.insert(PLAYER_COLLIDER);
        commands.insert(TnuaController::default());
        commands.insert(PlayerMotionConfig {
            speed: 16.0,
            sprint_speed_multiplier: 1.75,
            crouch_speed_multiplier: 0.75,
            walk: TnuaBuiltinWalk {
                float_height: PLAYER_FLOAT_HEIGHT_FROM_CENTER,
                max_slope: 80.0_f32.to_radians(),
                turning_angvel: Float::INFINITY,
                ..Default::default()
            },
            jump: TnuaBuiltinJump {
                height: 25.0,               // TODO change this back to 2.25 or maybe 2.75
                shorten_extra_gravity: 0.0, // Disable variable height jumps
                ..Default::default()
            },
            crouch: TnuaBuiltinCrouch {
                float_offset: -0.7,
                height_change_impulse_limit: 5.0,
                ..Default::default()
            },
            actions_in_air: 0,
        });
        commands.insert(ForwardFromCamera::default());
        commands.insert(TnuaCrouchEnforcer::new(0.5 * Vector3::Y, |cmd| {
            let bundle = TnuaAvian3dSensorShape(
                Collider::try_from_constructor(PLAYER_COLLIDER, None)
                    .expect("failed to create crouch enforcer collider"),
            );
            cmd.insert(bundle);
        }));
        commands.insert(TnuaSimpleAirActionsCounter::default());

        // commands.insert(Sleeping);
        // commands.insert(TnuaToggle::Disabled);
        // commands.insert(GravityScale(0.0));

        system_state.apply(world);
    }
}
