use bevy::{prelude::*, render::view::RenderLayers};

mod camera;
mod pickup;
pub mod weapons;

pub use camera::ViewModelCamera;
use camera::{NeedsRenderLayers, ViewModel, ViewModelPlugin};
pub use pickup::WeaponPickup;
use pickup::WeaponPickupPlugin;

use crate::render_layer;

/// Weapon spread radii, in degrees.
pub enum RangedSpread {
    Circle(f32),
    Ellipse(f32, f32),
}

pub enum RangedMode {
    Hitscan,
    Projectile {
        model: &'static str,
        velocity: f32,
        gravity: bool,
    },
}

pub enum WeaponAction {
    Ranged {
        spread: RangedSpread,
        mode: RangedMode,
        projectiles: usize,
    },
}

pub struct Weapon {
    pub name: &'static str,
    pub model: &'static str,
    pub action: WeaponAction,
    pub viewmodel_offset: Vec3,
}

#[derive(Component)]
pub struct PlayerWeapons {
    pub viewmodel_camera: Entity,
}

#[derive(Component)]
pub struct WeaponSlots {
    pub weapons: Vec<Option<&'static Weapon>>,
    pub current: usize,
    pub capacity: usize,
}
impl WeaponSlots {
    pub fn new(capacity: usize) -> Self {
        Self {
            weapons: vec![None; capacity],
            current: 0,
            capacity,
        }
    }

    pub fn first_empty_slot(&self) -> Option<usize> {
        for i in 0..self.capacity {
            if self.weapons[i].is_none() {
                return Some(i);
            }
        }
        None
    }

    pub fn equip(&mut self, weapon: &'static Weapon, slot: Option<usize>) -> Option<usize> {
        let Some(slot) = slot.or_else(|| self.first_empty_slot()) else {
            return None;
        };

        self.weapons[slot] = Some(weapon);

        Some(slot)
    }

    pub fn switch(&mut self, slot: usize) -> Option<&'static Weapon> {
        let Some(weapon) = self.weapons.get(slot) else {
            return None;
        };

        self.current = slot;

        return *weapon;
    }
}

#[derive(Event)]
pub struct SwitchWeaponEvent {
    pub shooter: Entity,
    pub slot: usize,
}

pub struct WeaponPlugin;

impl Plugin for WeaponPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((ViewModelPlugin, WeaponPickupPlugin));
        app.add_event::<SwitchWeaponEvent>();
        app.add_systems(Update, switch_weapons);
    }
}

fn switch_weapons(
    mut commands: Commands,
    mut events: EventReader<SwitchWeaponEvent>,
    mut weapons: Query<(&mut WeaponSlots, &PlayerWeapons)>,
    cameras: Query<Entity, With<ViewModelCamera>>,
    asset_server: Res<AssetServer>,
) {
    for event in events.read() {
        let Ok((mut slots, weapons)) = weapons.get_mut(event.shooter) else {
            continue;
        };
        let Ok(camera) = cameras.get(weapons.viewmodel_camera) else {
            continue;
        };

        commands.entity(camera).despawn_descendants();

        let Some(weapon) = slots.switch(event.slot) else {
            continue;
        };

        let child = commands
            .spawn((
                Transform::default(),
                ViewModel {
                    offset: weapon.viewmodel_offset,
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn((
                    Transform::from_translation(weapon.viewmodel_offset),
                    NeedsRenderLayers(RenderLayers::layer(render_layer::VIEW_MODEL)),
                    SceneRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(weapon.model))),
                ));
            })
            .id();

        commands.entity(camera).add_child(child);
    }
}
