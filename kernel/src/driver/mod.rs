//! Device drivers.
use macros::init;

pub mod keyboard;

/// Initializes the drivers of the devices found in the system. A device that
/// fails to initialize is only reported, since the kernel can run without it.
///
/// # Safety
/// This function must be called only once, during the initialization of the
/// kernel, once the interrupt controllers have been set up.
#[init]
pub unsafe fn setup() {
    #[cfg(target_arch = "x86_64")]
    if let Err(error) = keyboard::ps2::setup() {
        log::warn!("PS/2: keyboard unavailable: {error:?}");
    }
}
