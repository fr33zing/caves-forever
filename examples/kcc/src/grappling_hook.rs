use avian3d::prelude::*;
use bevy::prelude::*;

use crate::player::{Player, PlayerCamera, PlayerMotion, Section};

const GRAPPLING_HOOK_VELOCITY: f32 = 256.0;
const MISS_TIME: f64 = 1.0;
const SPRING_CONSTANT: f32 = 24.0;

#[derive(Component)]
pub struct HasGrapplingHook;

#[derive(Component)]
pub struct GrapplingHook {
    pub spawned: f64,
    pub hooked: bool,
}
impl GrapplingHook {
    pub fn new(time: &Res<Time>) -> Self {
        Self {
            spawned: time.elapsed_secs_f64(),
            hooked: false,
        }
    }
}

pub struct GrapplingHookPlugin;

impl Plugin for GrapplingHookPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                fire_or_remove.run_if(resource_changed::<ButtonInput<KeyCode>>),
                despawn_missed_hooks,
                attach_to_surface,
                debug,
                accelerate,
            ),
        );
    }
}

fn fire_or_remove(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    keyboard: Res<ButtonInput<KeyCode>>,
    player: Option<Single<(&GlobalTransform, &Section), With<Player>>>,
    camera: Option<Single<&Transform, With<PlayerCamera>>>,
    grappling_hook: Option<Single<(Entity, &GrapplingHook)>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyQ) {
        return;
    }

    if let Some(grappling_hook) = grappling_hook {
        let (entity, grappling_hook) = grappling_hook.into_inner();

        if !grappling_hook.hooked {
            return;
        }

        let mut commands = commands.entity(entity);
        commands.remove_parent();
        commands.despawn();

        return;
    }

    let Some(camera) = camera else {
        return;
    };
    let Some(player) = player else {
        return;
    };
    let (transform, section) = player.into_inner();

    commands.spawn((
        GrapplingHook::new(&time),
        Transform::from_translation(section.top(transform.translation()))
            .with_rotation(camera.rotation),
        RigidBody::Kinematic,
        Sensor,
        LinearVelocity(camera.forward() * GRAPPLING_HOOK_VELOCITY),
        Collider::sphere(0.2),
        SpeculativeMargin(0.0),
        SweptCcd::new_with_mode(SweepMode::Linear).include_dynamic(true),
        Mesh3d(meshes.add(Sphere::new(0.2))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 0.0, 0.0),
            ..default()
        })),
    ));
}

fn despawn_missed_hooks(
    mut commands: Commands,
    time: Res<Time>,
    grappling_hook: Option<Single<(Entity, &GrapplingHook)>>,
) {
    let Some(grappling_hook) = grappling_hook else {
        return;
    };
    let (entity, grappling_hook) = grappling_hook.into_inner();

    if grappling_hook.hooked {
        return;
    }

    let elapsed = time.elapsed_secs_f64() - grappling_hook.spawned;
    if elapsed > MISS_TIME {
        commands.entity(entity).despawn();
    }
}

fn attach_to_surface(
    mut commands: Commands,
    mut collisions: EventReader<Collision>,
    grappling_hook: Option<Single<(Entity, &mut GrapplingHook)>>,
    player: Option<Single<Entity, With<Player>>>,
    sensors: Query<(), With<Sensor>>,
) {
    let Some(player) = player else {
        return;
    };
    let Some(grappling_hook) = grappling_hook else {
        return;
    };
    let (entity, mut grappling_hook) = grappling_hook.into_inner();

    if grappling_hook.hooked {
        return;
    }

    for Collision(contacts) in collisions.read() {
        let Contacts {
            entity1,
            entity2,
            manifolds,
            ..
        } = contacts;

        if *entity1 != entity && *entity2 != entity {
            continue;
        }
        if *entity1 == *player || *entity2 == *player {
            continue;
        }
        if sensors.get(*entity1).is_ok() && sensors.get(*entity2).is_ok() {
            continue;
        }

        let Some(mut deepest_contact) = ({
            let mut deepest: Option<ContactData> = None;
            for manifold in manifolds {
                let d = manifold.find_deepest_contact();

                if let Some(ContactData { penetration, .. }) = deepest {
                    let Some(d) = d else {
                        continue;
                    };

                    if d.penetration > penetration {
                        deepest = Some(*d);
                    }
                } else {
                    deepest = d.copied();
                }
            }
            deepest
        }) else {
            return;
        };

        if *entity1 == entity {
            commands.entity(*entity2).add_child(entity);
        } else {
            commands.entity(*entity1).add_child(entity);
            deepest_contact.flip();
        }

        let mut commands = commands.entity(entity);
        commands.insert(Transform::from_translation(deepest_contact.point2));
        commands.insert(RigidBody::Static);
        commands.remove::<LinearVelocity>();
        grappling_hook.hooked = true;

        return;
    }
}

fn debug(
    mut gizmos: Gizmos,
    grappling_hook: Option<Single<(&GlobalTransform, &GrapplingHook)>>,
    player: Option<Single<(&Transform, &Section), With<Player>>>,
) {
    let Some(player) = player else {
        return;
    };
    let Some(grappling_hook) = grappling_hook else {
        return;
    };

    let (grappling_hook_transform, grappling_hook) = grappling_hook.into_inner();
    let (player_transform, section) = player.into_inner();

    gizmos.line(
        section.center(player_transform.translation),
        grappling_hook_transform.translation(),
        if grappling_hook.hooked {
            Color::srgb(0.0, 0.0, 0.0)
        } else {
            Color::srgb(0.0, 0.0, 1.0)
        },
    );
}

fn accelerate(
    time: Res<Time>,
    grappling_hook: Option<Single<(&GlobalTransform, &GrapplingHook)>>,
    player: Option<Single<(&Transform, &mut PlayerMotion)>>,
) {
    let Some(grappling_hook) = grappling_hook else {
        return;
    };
    let Some(player) = player else {
        return;
    };

    let (grappling_hook_transform, grappling_hook) = grappling_hook.into_inner();

    if !grappling_hook.hooked {
        return;
    }

    let (player_transform, mut player_motion) = player.into_inner();

    let distance = grappling_hook_transform
        .translation()
        .distance(player_transform.translation);
    let direction =
        (grappling_hook_transform.translation() - player_transform.translation) / distance;

    let force = SPRING_CONSTANT * distance;

    player_motion.no_gravity_this_frame = true;
    player_motion.forces.external += direction * 512.0 * time.delta_secs();
}
