use bevy::prelude::*;
use strum::{EnumIter, EnumProperty};

//
// Modes
//

#[derive(EnumProperty, EnumIter, Default, Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[repr(u8)]
pub enum EditorMode {
    #[default]
    #[strum(props(Name = "Tunnels"))]
    Tunnels = 0,

    #[strum(props(Name = "Rooms"))]
    Rooms = 1,
}

#[derive(EnumProperty, EnumIter, Default, Debug, PartialEq, Eq, Clone, Copy, Hash)]
#[repr(u8)]
pub enum EditorViewMode {
    #[default]
    #[strum(props(Name = "Editor"))]
    Editor = 0,

    #[strum(props(Name = "Preview"))]
    Preview = 1,
}

//
// Mode-specific states
//

#[derive(Debug)]
pub struct TunnelsModeState {
    pub mirror: bool,
}

impl Default for TunnelsModeState {
    fn default() -> Self {
        Self { mirror: true }
    }
}

//
// Main state
//

#[derive(Resource, Debug)]
pub struct EditorState {
    pub sensitivity: f32,
    pub mode: EditorMode,
    pub view: EditorViewMode,
    pub tunnels_mode: TunnelsModeState,
    pub filename_filter: String,
}

impl Default for EditorState {
    fn default() -> Self {
        Self {
            sensitivity: 1.0,
            mode: EditorMode::default(),
            view: EditorViewMode::default(),
            tunnels_mode: TunnelsModeState::default(),
            filename_filter: String::default(),
        }
    }
}
