use bevy::prelude::*;

mod door;
pub use door::*; //TEMP

pub struct MeshGenerationPlugin;

impl Plugin for MeshGenerationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, door::init_resources);
        app.add_systems(Update, (door::open_doors_on_contact, door::animate_doors));
    }
}
