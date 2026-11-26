//! The commands of the console that are specific to the kernel.
use core::fmt::Write;

use alloc::format;
use console::{
    Color, Console,
    command::{self, Command, Context, Error},
};

use crate::{
    driver::keyboard::layout::{self, Layout},
    scheduler,
    time::{duration::Duration, instant::Instant},
};

/// Registers the kernel commands into the console.
pub fn register<S: console::system::System>(console: &mut Console<S>) {
    for command in COMMANDS {
        console.register(*command);
    }
}

const COMMANDS: &[Command] = &[
    Command {
        name: "ps",
        summary: "List the threads",
        usage: "",
        handler: ps,
    },
    Command {
        name: "stress",
        summary: "Load the CPUs with busy threads for a while",
        usage: "<threads> [seconds]",
        handler: stress,
    },
    Command {
        name: "keyb",
        summary: "Display or change the keyboard layout",
        usage: "[us|fr]",
        handler: keyb,
    },
    Command {
        name: "reboot",
        summary: "Restart the computer",
        usage: "",
        handler: reboot,
    },
];

/// The maximum number of threads that `stress` can create at once.
const MAX_STRESS_THREADS: usize = 64;

/// The maximum duration of a `stress` run, in seconds.
const MAX_STRESS_SECONDS: u64 = 300;

#[allow(clippy::unnecessary_wraps)]
fn ps(ctx: &mut Context<'_>, _: &[&str]) -> Result<(), Error> {
    let tasks = scheduler::tasks();
    ctx.out
        .write_colored(Color::White, format_args!("{:>6}  {}\n", "ID", "STATE"));
    for (id, state) in &tasks {
        _ = writeln!(ctx.out, "{id:>6}  {state:?}");
    }
    _ = writeln!(ctx.out, "{} thread(s)", tasks.len());
    Ok(())
}

fn stress(ctx: &mut Context<'_>, args: &[&str]) -> Result<(), Error> {
    let (threads, seconds) = match args {
        [threads] => (command::parse::<usize>(threads)?, 5),
        [threads, seconds] => (
            command::parse::<usize>(threads)?,
            command::parse::<u64>(seconds)?,
        ),
        _ => return Err(Error::Usage),
    };
    if !(1..=MAX_STRESS_THREADS).contains(&threads) {
        return Err(Error::Failed(format!(
            "The number of threads must be between 1 and {MAX_STRESS_THREADS}"
        )));
    }
    if !(1..=MAX_STRESS_SECONDS).contains(&seconds) {
        return Err(Error::Failed(format!(
            "The duration must be between 1 and {MAX_STRESS_SECONDS} seconds"
        )));
    }

    // Each thread spins until the deadline, then exits. The threads are
    // spread over the CPUs by the scheduler.
    let deadline = Instant::now() + Duration::from_secs(seconds);
    for _ in 0..threads {
        scheduler::spawn(move || {
            while Instant::now() < deadline {
                core::hint::spin_loop();
            }
        });
    }
    _ = writeln!(
        ctx.out,
        "Started {threads} busy thread(s) for {seconds} second(s)"
    );
    Ok(())
}

fn keyb(ctx: &mut Context<'_>, args: &[&str]) -> Result<(), Error> {
    match args {
        [] => {
            let current = layout::current();
            _ = writeln!(
                ctx.out,
                "Current keyboard layout: {} ({})",
                current.name(),
                current.description()
            );
            _ = writeln!(ctx.out, "Available layouts:");
            for layout in Layout::ALL {
                _ = writeln!(ctx.out, "  {:<4} {}", layout.name(), layout.description());
            }
            Ok(())
        }
        [name] => {
            let layout = Layout::from_name(name)
                .ok_or_else(|| Error::Failed(format!("Unknown keyboard layout: {name}")))?;
            layout::set_current(layout);
            _ = writeln!(ctx.out, "Keyboard layout: {}", layout.description());
            Ok(())
        }
        _ => Err(Error::Usage),
    }
}

fn reboot(_: &mut Context<'_>, _: &[&str]) -> Result<(), Error> {
    log::info!("Rebooting...");
    #[cfg(target_arch = "x86_64")]
    crate::driver::keyboard::ps2::pulse_reset_line();
    Err(Error::Failed("The computer could not be restarted".into()))
}
