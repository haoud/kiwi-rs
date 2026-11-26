//! The kernel side of the debug console.
//!
//! The console itself (screen layout, line editing, history, commands and
//! system monitor) lives in the `console` crate, which does not depend on the
//! kernel. This module only provides the kernel implementations of the
//! abstractions it needs:
//! - [`backend::FramebufferBackend`] is a Ratatui backend drawing the cells
//!   of the terminal on the framebuffer with the embedded Terminus fonts;
//! - [`input::KeyboardInput`] turns the events of the keyboard driver into
//!   key presses, using the current keyboard layout;
//! - [`input::Interval`] wakes up the console periodically with a timer;
//! - [`system::KernelSystem`] collects the memory and CPU statistics;
//! - [`commands`] adds the commands that only make sense in the kernel;
//! - [`memory`] provides the memory displayed by the memory dump panel.
//!
//! The console runs as an asynchronous task on the executor of the CPU that
//! started it: it sleeps until a key is pressed or its refresh period elapses,
//! so it does not need a dedicated thread and costs nothing when idle.
use alloc::boxed::Box;
use console::{Console, panel::HexDumpPanel};

use crate::{
    future,
    time::duration::Duration,
    video::{Font, Framebuffer},
};

mod backend;
mod commands;
mod input;
mod memory;
mod system;

/// The period of the ticks of the console: the panels are animated at this
/// period, and the system information is refreshed less often (see
/// [`console::console::REFRESH_PERIOD`]).
const TICK_PERIOD: Duration = Duration::from_millis(50);

/// Starts the debug console on the framebuffer, as a task of the executor of
/// the current CPU. If there is no usable framebuffer, the console is not
/// started and a warning is logged.
pub fn start() {
    let framebuffer = match Framebuffer::take() {
        Ok(framebuffer) => framebuffer,
        Err(error) => {
            log::warn!("Debug console disabled: no usable framebuffer ({error:?})");
            return;
        }
    };

    let backend = backend::FramebufferBackend::new(framebuffer, Font::regular(), Font::bold());
    let size = console::ratatui::backend::Backend::size(&backend).unwrap_or_default();
    let (width, height) = (size.width, size.height);
    let mut console = Console::new(system::KernelSystem, width, height);
    commands::register(&mut console);
    let rodata = memory::kernel_rodata();
    console.add_panel(Box::new(
        HexDumpPanel::new("Memory dump · .rodata", rodata).starting_at(memory::first_text(rodata)),
    ));

    log::info!("Debug console started ({width}x{height} cells)");
    let keyboard = input::KeyboardInput::new();
    let ticker = input::Interval::new(TICK_PERIOD);
    future::spawn(async move {
        console.run(backend, keyboard, ticker).await;
    });
}
