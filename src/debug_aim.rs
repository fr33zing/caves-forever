use avian3d::prelude::*;
use bevy::prelude::*;

use crate::{tnua::IsPlayer, worldgen::terrain::DestroyTerrain};

pub struct DebugAimPlugin;

impl Plugin for DebugAimPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, update);
    }
}

fn update(
    spatial_query: SpatialQuery,
    camera_query: Query<&Transform, With<Camera>>,
    player_query: Query<Entity, With<IsPlayer>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut event: EventWriter<DestroyTerrain>,
) {
    if !buttons.just_pressed(MouseButton::Right) {
        return;
    }

    if let Ok(player) = player_query.get_single() {
        for camera in camera_query.iter() {
            let shape = Collider::sphere(0.25);
            let origin = camera.translation;
            let rotation = Quat::default();
            let direction = camera.forward();
            let config = ShapeCastConfig::from_max_distance(100.0);
            let filter = SpatialQueryFilter::from_excluded_entities([player]);

            if let Some(hit) =
                spatial_query.cast_shape(&shape, origin, rotation, direction, &config, &filter)
            {
                event.send(DestroyTerrain {
                    position: hit.point1,
                    radius: 6.0,
                    force: 16.0,
                });
            }
        }
    }
}
