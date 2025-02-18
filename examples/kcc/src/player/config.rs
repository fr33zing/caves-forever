use bevy::prelude::*;

#[derive(Resource)]
pub struct PlayerConfig {
    pub height: f32,
    pub radius: f32,
}

#[derive(Resource)]
pub struct PlayerKeybinds {
    pub switch_camera: Option<Keybind>,
    pub forward: Option<Keybind>,
    pub backward: Option<Keybind>,
    pub left: Option<Keybind>,
    pub right: Option<Keybind>,
    pub jump: Option<Keybind>,
    pub sprint: Option<Keybind>,

    #[cfg(feature = "crouch")]
    pub crouch: Option<Keybind>,
}

pub enum Keybind {
    Keyboard(KeyCode),
    Mouse(MouseButton),
}

#[derive(Resource)]
pub struct PlayerCameraConfig {
    pub mode: PlayerCameraMode,
    pub eye_offset: f32,
    pub sensitivity: f32,
    pub fov_degrees: f32,
    pub third_person_distance: f32,
}

#[derive(Default)]
pub enum PlayerCameraMode {
    #[default]
    FirstPerson,
    ThirdPerson,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        Self {
            height: 1.8288, // 6'
            radius: 0.4572, // 1'6"
        }
    }
}

impl Default for PlayerKeybinds {
    fn default() -> Self {
        Self {
            switch_camera: Some(Keybind::Mouse(MouseButton::Middle)),
            forward: Some(Keybind::Keyboard(KeyCode::KeyW)),
            backward: Some(Keybind::Keyboard(KeyCode::KeyS)),
            left: Some(Keybind::Keyboard(KeyCode::KeyA)),
            right: Some(Keybind::Keyboard(KeyCode::KeyD)),
            jump: Some(Keybind::Keyboard(KeyCode::Space)),
            sprint: Some(Keybind::Keyboard(KeyCode::ShiftLeft)),

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
            mode: default(),
            eye_offset: 0.1524, // 6"
            sensitivity: 1.0,
            fov_degrees: 45.0,
            third_person_distance: 8.0,
        }
    }
}
