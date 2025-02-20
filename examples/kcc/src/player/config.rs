use bevy::prelude::*;

use super::actions::PlayerAction;

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
}

#[derive(Resource)]
pub struct PlayerActionsConfig {
    pub jump: Option<JumpActionConfig>,
    pub crouch: Option<CrouchActionConfig>,
    pub slide: Option<SlideActionConfig>,
}

pub struct JumpActionConfig {
    pub force: f32,
    pub bufferable: bool,
    pub buffer_distance: f32,
    pub buffer_expiry_secs: f64,
}

pub struct CrouchActionConfig {
    pub transition_speed: f32,
    pub crouchjump_additional_clearance: bool,
    pub slide_if_running: bool,
}

pub struct SlideActionConfig {
    /// The initial push applied to the player when they start sliding.
    pub force: f32,
    /// The player cannot stop sliding if they are moving faster than this.
    /// Gravity doesn't count.
    pub stop_sliding_velocity: f32,
    /// Additional acceleration applied to the player when the ground slope
    /// is at least `max_acceleration_slope_degrees`.
    pub max_slope_acceleration: f32,
    /// Any slope steeper than this applies adds additional acceleration beyond the maximum.
    pub max_acceleration_slope_degrees: f32,
    /// Any slope shallower than this is considered flat, applying no acceleration.
    pub min_acceleration_slope_degrees: f32,
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
    pub jump: Option<Keybind>,
    pub crouch: Option<Keybind>,

    /// Run, unless [PlayerInputConfig.always_run], then it's walk.
    pub walk_mod: Option<Keybind>,

    #[cfg(feature = "camera")]
    pub switch_camera: Option<Keybind>,
}

pub enum Keybind {
    Keyboard(KeyCode),
    Mouse(MouseButton),
}

#[cfg(feature = "camera")]
#[derive(Resource)]
pub struct PlayerCameraConfig {
    pub mode: PlayerCameraMode,
    pub allowed_modes: Vec<PlayerCameraMode>,
    pub eye_offset: f32,
    pub sensitivity: f32,
    pub fov_degrees: f32,
    pub third_person_distance: f32,
}

#[cfg(feature = "camera")]
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
        }
    }
}

impl PlayerActionsConfig {
    pub fn expiry_for(&self, action: &PlayerAction) -> Option<f64> {
        match action {
            PlayerAction::Jump => self.jump.as_ref().map(|j| j.buffer_expiry_secs),
            _ => None,
        }
    }
}

impl Default for PlayerActionsConfig {
    fn default() -> Self {
        Self {
            jump: Some(default()),
            crouch: Some(default()),
            slide: Some(default()),
        }
    }
}

impl Default for JumpActionConfig {
    fn default() -> Self {
        Self {
            force: 16.0,
            bufferable: true,
            buffer_distance: 1.5,
            buffer_expiry_secs: 0.5,
        }
    }
}

impl Default for CrouchActionConfig {
    fn default() -> Self {
        Self {
            transition_speed: 12.0,
            crouchjump_additional_clearance: true,
            slide_if_running: true,
        }
    }
}

impl Default for SlideActionConfig {
    fn default() -> Self {
        Self {
            force: 48.0,
            stop_sliding_velocity: 22.0,
            max_slope_acceleration: 400.0,
            max_acceleration_slope_degrees: 45.0,
            min_acceleration_slope_degrees: 1.0,
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
            jump: Some(Keybind::Keyboard(KeyCode::Space)),
            crouch: Some(Keybind::Keyboard(KeyCode::ControlLeft)),

            #[cfg(feature = "camera")]
            switch_camera: Some(Keybind::Mouse(MouseButton::Middle)),
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

#[cfg(feature = "camera")]
impl Default for PlayerCameraConfig {
    fn default() -> Self {
        Self {
            mode: default(),
            allowed_modes: vec![PlayerCameraMode::FirstPerson, PlayerCameraMode::ThirdPerson],
            eye_offset: 0.1524, // 6"
            sensitivity: 1.0,
            fov_degrees: 45.0,
            third_person_distance: 8.0,
        }
    }
}
