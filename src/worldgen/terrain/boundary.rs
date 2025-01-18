use avian3d::prelude::*;
use bevy::prelude::*;
use bevy_tnua::TnuaToggle;

use super::CHUNK_SIZE_F;

#[derive(Component)]
pub struct IntersectsBoundary;

#[derive(Component)]
pub struct LoadingBoundary {
    aabb: ColliderAabb,
}

impl LoadingBoundary {
    pub fn new(chunk_pos: IVec3) -> Self {
        let center = chunk_pos.as_vec3() * CHUNK_SIZE_F;
        let half = CHUNK_SIZE_F / 2.0;

        Self {
            aabb: ColliderAabb {
                min: center - half,
                max: center + half,
            },
        }
    }
}

pub fn enforce_loading_chunk_boundaries(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    boundaries: Query<&LoadingBoundary>,
    intersecting: Query<Entity, With<IntersectsBoundary>>,
) {
    intersecting.iter().for_each(|entity| {
        let mut commands = commands.entity(entity);
        commands.remove::<GravityScale>();
        commands.remove::<TnuaToggle>();
        commands.remove::<Sleeping>();
        commands.remove::<IntersectsBoundary>();
    });

    boundaries.iter().for_each(|boundary| {
        spatial_query
            .aabb_intersections_with_aabb(boundary.aabb)
            .iter()
            .for_each(|entity| {
                let mut commands = commands.entity(*entity);
                commands.insert(GravityScale(0.0));
                commands.insert(TnuaToggle::Disabled);
                commands.insert(Sleeping);
                commands.insert(IntersectsBoundary);
            });
    });
}
