use avian3d::prelude::*;
use bevy::{prelude::*, utils::hashbrown::HashSet};
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
        let world_pos = chunk_pos.as_vec3() * CHUNK_SIZE_F;
        Self {
            aabb: ColliderAabb {
                min: world_pos,
                max: world_pos + CHUNK_SIZE_F,
            },
        }
    }
}

pub fn enforce_loading_chunk_boundaries(
    mut commands: Commands,
    spatial_query: SpatialQuery,
    boundaries: Query<&LoadingBoundary>,
    intersecting_prev: Query<Entity, With<IntersectsBoundary>>,
) {
    let mut intersecting_curr = HashSet::<Entity>::new();

    boundaries.iter().for_each(|boundary| {
        spatial_query
            .aabb_intersections_with_aabb(boundary.aabb)
            .iter()
            .for_each(|entity| {
                intersecting_curr.insert(*entity);

                if intersecting_prev.contains(*entity) {
                    return;
                }

                let mut commands = commands.entity(*entity);
                commands.insert(GravityScale(0.0));
                commands.insert(TnuaToggle::Disabled);
                commands.insert(Sleeping);
                commands.insert(IntersectsBoundary);
            });
    });

    intersecting_prev.iter().for_each(|entity| {
        if intersecting_curr.contains(&entity) {
            return;
        }

        let mut commands = commands.entity(entity);
        commands.remove::<GravityScale>();
        commands.remove::<TnuaToggle>();
        commands.remove::<Sleeping>();
        commands.remove::<IntersectsBoundary>();
    });
}
