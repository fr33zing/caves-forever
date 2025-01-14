use avian3d::prelude::*;

#[derive(PhysicsLayer, Default, Clone, Copy, Debug)]
pub enum GameLayer {
    #[default]
    World,
    Brush,
    Cable,
    Player,
    Enemy,
}

//pub const BRUSH_ONLY: SpatialQueryFilter = SpatialQueryFilter::from_mask(GameLayer::Brush);
