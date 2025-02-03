use std::{fs::File, io::Read};

use bevy::prelude::*;

use super::asset::AssetCollection;

pub struct LayoutPlugin;

impl Plugin for LayoutPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, load_asset_collection);
    }
}

fn load_asset_collection(mut commands: Commands) {
    let path = if cfg!(debug_assertions) {
        "./assets/worldgen.staging.cbor"
    } else {
        "./assets/worldgen.production.cbor"
    };

    let mut file = File::open(path).expect("worldgen asset collection does not exist");
    let mut vec = Vec::new();
    file.read_to_end(&mut vec)
        .expect("failed to read worldgen asset collection");
    let assets: AssetCollection =
        cbor4ii::serde::from_slice(&vec).expect("failed to deserialize worldgen asset collection");

    commands.insert_resource(assets);
}
