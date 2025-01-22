use core::f32;
use std::f32::consts::PI;

use bevy::prelude::*;
use nalgebra::Point2;
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
    pub profile: Vec<Point2<f32>>,
    pub drag_point: Option<usize>,
    pub drag_start: Option<(Point2<f32>, Vec2)>,
}

impl Default for TunnelsModeState {
    fn default() -> Self {
        let n = 10;
        let radius = 5.0;
        let mut profile = Vec::new();

        for i in 0..n {
            let radians = (i as f32 / n as f32) * PI * 2.0;
            profile.push(Point2::new(radians.sin(), -radians.cos()) * radius);
        }

        Self {
            mirror: true,
            profile,
            drag_point: None,
            drag_start: None,
        }
    }
}

//
// Main state
//

#[derive(Resource, Default, Debug)]
pub struct EditorState {
    pub mode: EditorMode,
    pub view: EditorViewMode,
    pub tunnels_mode: TunnelsModeState,
}
