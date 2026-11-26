use crate::arch::addr::{Kernel, Virtual};

/// The layout of a pixel in a framebuffer: the position and the size, in
/// bits, of each color component inside the pixel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelFormat {
    pub bits_per_pixel: u16,
    pub red_shift: u8,
    pub red_size: u8,
    pub green_shift: u8,
    pub green_size: u8,
    pub blue_shift: u8,
    pub blue_size: u8,
}

/// A linear framebuffer, set up by the firmware or the bootloader before the
/// kernel was started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FramebufferInfo {
    /// The address of the first pixel of the framebuffer, which is mapped
    /// into the kernel address space.
    pub address: Virtual<Kernel>,

    /// The width of the framebuffer, in pixels.
    pub width: usize,

    /// The height of the framebuffer, in pixels.
    pub height: usize,

    /// The number of bytes between the start of two consecutive lines.
    pub pitch: usize,

    /// The layout of a pixel.
    pub format: PixelFormat,
}

/// Returns the framebuffer provided by the firmware or the bootloader, if
/// there is one with a direct RGB color model.
#[must_use]
pub fn framebuffer() -> Option<FramebufferInfo> {
    crate::arch::target::framebuffer::framebuffer()
}
