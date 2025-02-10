use bevy::prelude::*;

mod door;
pub use door::*; //TEMP

pub struct MeshGenerationPlugin;

impl Plugin for MeshGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DoorAnimationCurves>();
        app.add_systems(Update, (door::open_doors_on_contact, door::animate_doors));
    }
}
