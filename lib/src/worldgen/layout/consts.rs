/// Distance that a new sequence of rooms is placed away from the previous sequence.
pub const SEQUENCE_DISTANCE: f32 = 128.0;

/// Rooms will be placed at least this far apart from obstacles.
pub const ROOM_SHYNESS: f32 = 16.0;

/// Paths between rooms will route at least this far away from obstacles.
pub const TUNNEL_SHYNESS: f32 = 24.0;

/// Distance colliders are depenetrated by per iteration when arranging them.
pub const DEPENETRATION_DISTANCE: f32 = 4.0;

/// Number of points per unit of volume in the navigable hull between two rooms.
pub const HULL_DENSITY: f32 = 0.00001;

/// Distance considered to a be short hop when pathfinding between portals.
pub const SHORT_HOP: f32 = 24.0;

pub const TRIGGER_OFFSET: f32 = 8.0;
