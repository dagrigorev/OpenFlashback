//! Clean-room readers for the data files of Flashback (Delphine Software, 1992), DOS version.
//!
//! Nothing here contains game content: every reader works on the files of the user's own copy.

pub mod anim;
pub mod data;
pub mod level;
pub mod obj;
pub mod palette;
pub mod pge;
pub mod screen;
pub mod sprite;
pub mod unpack;

/// Width of the game screen in pixels.
pub const SCREEN_W: usize = 256;
/// Height of the game screen in pixels.
pub const SCREEN_H: usize = 224;

pub use data::DataDir;
pub use palette::{Palette, Rgb};
