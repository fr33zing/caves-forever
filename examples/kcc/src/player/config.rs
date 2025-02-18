use bevy::prelude::*;

#[derive(Resource)]
pub struct PlayerConfig {
    pub height: f32,
    pub radius: f32,
}

#[derive(Resource)]
pub struct PlayerMotionConfig {
    // TODO terminal_velocity: f32
    pub max_slope_degrees: f32,
    pub snap_to_ground_distance: f32,
    pub collide_and_slide_bounces: u8,
    pub skin: f32,
    pub gravity: f32,
    pub push_force: f32,

    #[cfg(feature = "jump")]
    pub jump_force: f32,

    #[cfg(feature = "jump")]
    pub jump_buffer_distance: f32,
}

#[derive(Resource)]
pub struct PlayerKeybinds {
    pub forward: Option<Keybind>,
    pub backward: Option<Keybind>,
    pub left: Option<Keybind>,
    pub right: Option<Keybind>,
    pub sprint: Option<Keybind>,

    #[cfg(feature = "jump")]
    pub jump: Option<Keybind>,

    #[cfg(all(feature = "first-person-camera", feature = "third-person-camera"))]
    pub switch_camera: Option<Keybind>,

    #[cfg(feature = "crouch")]
    pub crouch: Option<Keybind>,
}

pub enum Keybind {
    Keyboard(KeyCode),
    Mouse(MouseButton),
}

#[cfg(any(feature = "first-person-camera", feature = "third-person-camera"))]
#[derive(Resource)]
pub struct PlayerCameraConfig {
    pub eye_offset: f32,
    pub sensitivity: f32,
    pub fov_degrees: f32,

    #[cfg(all(feature = "first-person-camera", feature = "third-person-camera"))]
    pub mode: PlayerCameraMode,

    #[cfg(feature = "third-person-camera")]
    pub third_person_distance: f32,
}

#[cfg(all(feature = "first-person-camera", feature = "third-person-camera"))]
#[derive(Default)]
pub enum PlayerCameraMode {
    #[default]
    FirstPerson,
    ThirdPerson,
}

//
// Implementations
//

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            height: 1.8288, // 6'
            radius: 0.4572, // 1'6"
        }
    }
}

impl Default for PlayerMotionConfig {
    fn default() -> Self {
        Self {
            max_slope_degrees: 50.0,
            snap_to_ground_distance: 0.1,
            collide_and_slide_bounces: 3,
            skin: 0.005,
            gravity: 64.0,
            push_force: 28.0,

            #[cfg(feature = "jump")]
            jump_force: 16.0,
            #[cfg(feature = "jump")]
            jump_buffer_distance: 1.5,
        }
    }
}

impl Default for PlayerKeybinds {
    fn default() -> Self {
        Self {
            forward: Some(Keybind::Keyboard(KeyCode::KeyW)),
            backward: Some(Keybind::Keyboard(KeyCode::KeyS)),
            left: Some(Keybind::Keyboard(KeyCode::KeyA)),
            right: Some(Keybind::Keyboard(KeyCode::KeyD)),
            sprint: Some(Keybind::Keyboard(KeyCode::ShiftLeft)),

            #[cfg(all(feature = "first-person-camera", feature = "third-person-camera"))]
            switch_camera: Some(Keybind::Mouse(MouseButton::Middle)),

            #[cfg(feature = "jump")]
            jump: Some(Keybind::Keyboard(KeyCode::Space)),

            #[cfg(feature = "crouch")]
            crouch: Some(Keybind::Keyboard(KeyCode::ControlLeft)),
        }
    }
}

impl Keybind {
    pub fn pressed(
        &self,
        keyboard: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) -> bool {
        match self {
            Keybind::Keyboard(key_code) => keyboard.pressed(*key_code),
            Keybind::Mouse(mouse_button) => mouse.pressed(*mouse_button),
        }
    }

    pub fn just_pressed(
        &self,
        keyboard: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) -> bool {
        match self {
            Keybind::Keyboard(key_code) => keyboard.just_pressed(*key_code),
            Keybind::Mouse(mouse_button) => mouse.just_pressed(*mouse_button),
        }
    }

    pub fn just_released(
        &self,
        keyboard: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) -> bool {
        match self {
            Keybind::Keyboard(key_code) => keyboard.just_released(*key_code),
            Keybind::Mouse(mouse_button) => mouse.just_released(*mouse_button),
        }
    }
}

impl Default for PlayerCameraConfig {
    fn default() -> Self {
        Self {
            eye_offset: 0.1524, // 6"
            sensitivity: 1.0,
            fov_degrees: 45.0,

            #[cfg(all(feature = "first-person-camera", feature = "third-person-camera"))]
            mode: default(),

            #[cfg(feature = "third-person-camera")]
            third_person_distance: 8.0,
        }
    }
}
