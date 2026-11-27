use usize_cast::IntoUsize;

use crate::arch::{
    addr::{Kernel, Virtual},
    framebuffer::{FramebufferInfo, PixelFormat},
};

/// Request the bootloader to set up a graphical mode and to provide the
/// framebuffers it has set up.
static LIMINE_FRAMEBUFFER_REQUEST: limine::request::FramebufferRequest =
    limine::request::FramebufferRequest::new();

/// Returns the first framebuffer provided by the bootloader that uses a direct
/// RGB color model, or `None` if there is no such framebuffer.
#[must_use]
pub fn framebuffer() -> Option<FramebufferInfo> {
    LIMINE_FRAMEBUFFER_REQUEST
        .get_response()?
        .framebuffers()
        .find(|fb| fb.memory_model() == limine::framebuffer::MemoryModel::RGB)
        .map(|fb| FramebufferInfo {
            // The bootloader maps the framebuffer in the HHDM, and the address
            // it gives is already the virtual address of the framebuffer.
            address: Virtual::<Kernel>::new(fb.addr().addr()),
            width: fb.width().into_usize(),
            height: fb.height().into_usize(),
            pitch: fb.pitch().into_usize(),
            format: PixelFormat {
                bits_per_pixel: fb.bpp(),
                red_shift: fb.red_mask_shift(),
                red_size: fb.red_mask_size(),
                green_shift: fb.green_mask_shift(),
                green_size: fb.green_mask_size(),
                blue_shift: fb.blue_mask_shift(),
                blue_size: fb.blue_mask_size(),
            },
        })
}
