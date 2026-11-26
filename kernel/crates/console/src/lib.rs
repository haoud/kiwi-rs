//! A debug console with a modern text user interface, built with Ratatui.
//!
//! This crate contains the whole logic of the console (screen layout, line
//! editing, command history, command execution and system monitoring), but
//! does not know anything about the kernel. It only interacts with the outside
//! world through a few minimal abstractions:
//! - [`ratatui::backend::Backend`]: the terminal where the console is drawn.
//! - [`input::Input`]: an asynchronous source of key presses.
//! - [`input::Ticker`]: an asynchronous periodic event, used to refresh the
//!   system information and to blink the cursor.
//! - [`system::System`]: a provider of information about the system (memory
//!   usage, CPU times...).
//!
//! The console can be extended with new commands (see [`command`]) and new
//! panels in its sidebar (see [`panel`]).
#![no_std]

extern crate alloc;

pub mod builtins;
pub mod command;
pub mod console;
pub mod editor;
pub mod format;
pub mod input;
pub mod panel;
pub mod scrollback;
pub mod system;
pub mod theme;

pub use console::Console;
pub use ratatui;
