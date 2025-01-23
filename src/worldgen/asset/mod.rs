use serde::{Deserialize, Serialize};

mod tunnel;
use strum::{EnumIter, EnumProperty};
pub use tunnel::*;

#[repr(u8)]
#[derive(EnumIter, EnumProperty, Serialize, Deserialize, Clone, Copy, Debug, Eq, PartialEq)]
pub enum Environment {
    /// Asset will be used in release mode and debug mode.
    #[strum(props(Name = "Production"))]
    Production = 0,

    /// Asset will be used in debug mode.
    #[strum(props(Name = "Staging"))]
    Staging = 1,

    /// Asset will not be used.
    #[strum(props(Name = "Development"))]
    Development = 2,
}
