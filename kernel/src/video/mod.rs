//! Graphical output: the framebuffer set up by the bootloader, and the bitmap
//! fonts used to draw text on it.
pub mod font;
pub mod framebuffer;

pub use font::Font;
pub use framebuffer::{Framebuffer, Rgb};
