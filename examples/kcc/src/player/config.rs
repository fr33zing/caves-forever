use bevy::prelude::*;

#[cfg(feature = "actions")]
use super::input::PlayerAction;

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
    pub run_speed_mod: f32,
    pub friction: f32,
    pub friction_delay_secs: f64,
    pub ground_accelerate: f32,
    pub air_accelerate: f32,
    pub max_velocity_ground: f32,
    pub max_velocity_air: f32,

    #[cfg(feature = "jump")]
    pub jump_force: f32,
}

#[cfg(feature = "actions")]
#[derive(Resource)]
pub struct PlayerBufferedActionsConfig {
    /// Player must be within this distance to the ground in order to buffer a jump.
    #[cfg(feature = "jump")]
    pub jump_buffer_distance: f32,
    #[cfg(feature = "jump")]
    pub jump_expiry_secs: f64,
}

#[derive(Resource, Default)]
pub struct PlayerInputConfig {
    /// Run by default. The run key becomes the walk key.
    pub always_run: bool,
    pub walk_mod_mode: PlayerWalkModMode,
    pub binds: PlayerKeybinds,
}

#[derive(Default, PartialEq)]
pub enum PlayerWalkModMode {
    /// Walk mod is only on when the walk mod key is pressed.
    #[default]
    Hold,
    /// Pressing the walk mod key toggles walk mod.
    /// Releasing all movement keys *DOES NOT* toggle it back.
    Toggle,
    /// Pressing the walk mod key toggles walk mod.
    /// Releasing all movement keys *DOES* toggle it back.
    ToggleHybrid,
    /// Pressing the walk mod key enables walk mod, but doesn't disable it.
    /// Releasing all movement keys *DOES* toggle it back.
    Hybrid,
}

pub struct PlayerKeybinds {
    pub forward: Option<Keybind>,
    pub backward: Option<Keybind>,
    pub left: Option<Keybind>,
    pub right: Option<Keybind>,

    /// Run, unless [PlayerInputConfig.always_run], then it's walk.
    pub walk_mod: Option<Keybind>,

    #[cfg(all(feature = "first-person-camera", feature = "third-person-camera"))]
    pub switch_camera: Option<Keybind>,

    #[cfg(feature = "jump")]
    pub jump: Option<Keybind>,

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
        const QUAKE_UNITS_PER_METER: f32 = 16.0;

        Self {
            max_slope_degrees: 50.0,
            snap_to_ground_distance: 0.1,
            collide_and_slide_bounces: 3,
            skin: 0.005,
            gravity: 64.0,
            push_force: 28.0,
            run_speed_mod: 2.0,

            // Ratios based on Quake/QW/server/sv_phys.c
            // Not sure how 1:1 this is with Quake.
            friction: 6.0,
            friction_delay_secs: 1.0 / 20.0,
            // These are all halved due to run_speed_mod.
            ground_accelerate: 5.0 * QUAKE_UNITS_PER_METER,
            air_accelerate: 0.35 * QUAKE_UNITS_PER_METER,
            max_velocity_ground: 160.0 / QUAKE_UNITS_PER_METER,
            max_velocity_air: 160.0 / QUAKE_UNITS_PER_METER,

            #[cfg(feature = "jump")]
            jump_force: 16.0,
        }
    }
}

#[cfg(feature = "actions")]
impl PlayerBufferedActionsConfig {
    pub fn expiry_for(&self, action: &PlayerAction) -> Option<f64> {
        match action {
            #[cfg(feature = "jump")]
            PlayerAction::Jump => Some(self.jump_expiry_secs),
            _ => None,
        }
    }
}

#[cfg(feature = "actions")]
impl Default for PlayerBufferedActionsConfig {
    fn default() -> Self {
        Self {
            #[cfg(feature = "jump")]
            jump_buffer_distance: 1.5,
            #[cfg(feature = "jump")]
            jump_expiry_secs: 0.5,
        }
    }
}

impl PlayerKeybinds {
    pub fn any_pressed<const N: usize>(
        binds: [&Option<Keybind>; N],
        keyboard: &ButtonInput<KeyCode>,
        mouse: &ButtonInput<MouseButton>,
    ) -> bool {
        binds.iter().any(|bind| {
            let Some(bind) = bind else {
                return false;
            };
            bind.pressed(keyboard, mouse)
        })
    }
}

impl Default for PlayerKeybinds {
    fn default() -> Self {
        Self {
            forward: Some(Keybind::Keyboard(KeyCode::KeyW)),
            backward: Some(Keybind::Keyboard(KeyCode::KeyS)),
            left: Some(Keybind::Keyboard(KeyCode::KeyA)),
            right: Some(Keybind::Keyboard(KeyCode::KeyD)),
            walk_mod: Some(Keybind::Keyboard(KeyCode::ShiftLeft)),

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

#[cfg(any(feature = "first-person-camera", feature = "third-person-camera"))]
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
