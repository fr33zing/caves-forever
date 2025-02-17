use std::f32::consts::PI;

use avian3d::prelude::*;
use bevy::prelude::*;

use super::{SwitchWeaponEvent, Weapon, WeaponSlots};

#[derive(Resource)]
pub struct PickupSfx(pub Handle<AudioSource>);

#[derive(Component)]
pub struct WeaponPickup {
    pub weapon: &'static Weapon,
    pub active: bool,
}
impl WeaponPickup {
    pub fn new(weapon: &'static Weapon) -> Self {
        Self {
            weapon,
            active: true,
        }
    }
}

#[derive(Component)]
pub struct WeaponPickupChild;

pub struct WeaponPickupPlugin;

impl Plugin for WeaponPickupPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, setup);
        app.add_systems(Update, (add_required_components, animate, pickup));
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.insert_resource(PickupSfx(asset_server.load("sfx/pickup.ogg")))
}

fn add_required_components(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pickups: Query<(Entity, &WeaponPickup), Added<WeaponPickup>>,
) {
    pickups.iter().for_each(|(entity, pickup)| {
        let child = commands
            .spawn((
                WeaponPickupChild,
                Transform::default(),
                SceneRoot(
                    asset_server.load(GltfAssetLabel::Scene(0).from_asset(pickup.weapon.model)),
                ),
            ))
            .id();

        let mut commands = commands.entity(entity);
        commands.add_child(child);
        commands.insert((
            Collider::capsule_endpoints(0.65, Vec3::ZERO, Vec3::Y * 2.0),
            Sensor,
        ));
        commands.insert_if_new(Transform::default());
        commands.insert_if_new(Visibility::Visible);
    });
}

fn animate(time: Res<Time>, mut pickups: Query<&mut Transform, With<WeaponPickupChild>>) {
    const SECONDS_PER_ROTATION: f32 = 5.0;
    const SECONDS_PER_BOUNCE: f32 = 0.65;
    const Y: f32 = 1.35;
    const BOUNCE_HEIGHT: f32 = 0.125;
    const CIRCLE: f32 = PI * 2.0;

    pickups.iter_mut().for_each(|mut pickup| {
        pickup.translation.y = Y
            + (time.elapsed_secs_wrapped() / SECONDS_PER_BOUNCE)
                .sin()
                .abs()
                * BOUNCE_HEIGHT;
        pickup.rotation = Quat::from_euler(
            EulerRot::YXZ,
            ((time.elapsed_secs_wrapped() / SECONDS_PER_ROTATION) * CIRCLE) % CIRCLE,
            0.0,
            0.0,
        );
    });
}

fn pickup(
    sfx: Res<PickupSfx>,
    mut commands: Commands,
    mut collisions: EventReader<CollisionStarted>,
    mut switch_weapons: EventWriter<SwitchWeaponEvent>,
    mut slots: Query<(Entity, &mut WeaponSlots)>,
    mut pickups: Query<(Entity, &mut WeaponPickup)>,
) {
    for CollisionStarted(entity1, entity2) in collisions.read() {
        let ((pickup_entity, mut pickup), (shooter, mut slots)) =
            match (pickups.get_mut(*entity1), slots.get_mut(*entity2)) {
                (Ok(pickup), Ok(shooter)) => (pickup, shooter),
                _ => match (pickups.get_mut(*entity2), slots.get_mut(*entity1)) {
                    (Ok(pickup), Ok(shooter)) => (pickup, shooter),
                    _ => continue,
                },
            };
        if !pickup.active {
            continue;
        };

        let Some(slot) = slots.equip(pickup.weapon, None) else {
            continue;
        };

        pickup.active = false;
        commands.entity(pickup_entity).despawn_recursive();
        commands.spawn((AudioPlayer::new(sfx.0.clone()), PlaybackSettings::DESPAWN));
        switch_weapons.send(SwitchWeaponEvent { shooter, slot });
    }
}
