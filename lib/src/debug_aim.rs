use avian3d::prelude::*;
use bevy::{prelude::*, window::PrimaryWindow};

use crate::{player::IsPlayer, worldgen::terrain::DestroyTerrainEvent};

pub struct DebugAimPlugin;

impl Plugin for DebugAimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update);
    }
}

fn update(
    spatial_query: SpatialQuery,
    camera_query: Query<&Transform, With<Camera>>,
    player: Single<Entity, With<IsPlayer>>,
    buttons: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut event: EventWriter<DestroyTerrainEvent>,
) {
    if !buttons.just_pressed(MouseButton::Left) || window.cursor_options.visible {
        return;
    }

    // TODO make this only run for the player's main camera
    for camera in camera_query.iter() {
        let shape = Collider::sphere(0.2);
        let origin = camera.translation;
        let rotation = Quat::default();
        let direction = camera.forward();
        let config = ShapeCastConfig::from_max_distance(100.0);
        let filter = SpatialQueryFilter::from_excluded_entities([*player]);

        if let Some(hit) =
            spatial_query.cast_shape(&shape, origin, rotation, direction, &config, &filter)
        {
            event.send(DestroyTerrainEvent {
                position: hit.point1,
                radius: 2.0,
                force: 1.0,
            });
        }
    }
}
