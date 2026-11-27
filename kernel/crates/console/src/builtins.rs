//! The commands provided by the console itself. They only rely on the
//! abstractions of this crate, so they are available on every system.

// All the handlers must have the signature of `command::Handler`, even those
// that never fail.
#![allow(clippy::unnecessary_wraps)]
use core::fmt::Write;

use crate::{
    command::{Command, Context, Error, Registry},
    format::{Bytes, Percent, Uptime},
    theme,
};

/// Registers all the builtin commands.
pub fn register(registry: &mut Registry) {
    for command in COMMANDS {
        registry.register(*command);
    }
}

/// The builtin commands, in no particular order: the registry sorts them by
/// name.
const COMMANDS: &[Command] = &[
    Command {
        name: "help",
        summary: "List the commands, or describe one command",
        usage: "[command]",
        handler: help,
    },
    Command {
        name: "cls",
        summary: "Clear the screen",
        usage: "",
        handler: cls,
    },
    Command {
        name: "echo",
        summary: "Display a message",
        usage: "[message...]",
        handler: echo,
    },
    Command {
        name: "history",
        summary: "Display the command history",
        usage: "",
        handler: history,
    },
    Command {
        name: "mem",
        summary: "Display the memory usage",
        usage: "",
        handler: mem,
    },
    Command {
        name: "cpu",
        summary: "Display the load of each CPU",
        usage: "",
        handler: cpu,
    },
    Command {
        name: "uptime",
        summary: "Display the time elapsed since boot",
        usage: "",
        handler: uptime,
    },
    Command {
        name: "ver",
        summary: "Display the version of the system",
        usage: "",
        handler: ver,
    },
];

/// Lists all the commands with their summary, or describes the command given as
/// argument with its summary and its usage.
fn help(ctx: &mut Context<'_>, args: &[&str]) -> Result<(), Error> {
    match args {
        [] => {
            let width = ctx.commands.iter().map(|c| c.name.len()).max().unwrap_or(0);
            for command in ctx.commands.iter() {
                ctx.out
                    .write_bold(theme::INFO, format_args!("  {:width$}", command.name));
                ctx.out
                    .write_colored(theme::MUTED, format_args!("  {}\n", command.summary));
            }
            _ = writeln!(ctx.out);
            ctx.out.write_colored(
                theme::MUTED,
                format_args!("Type `help <command>` for more information on a command.\n"),
            );
            Ok(())
        }
        [name] => {
            let command = ctx
                .commands
                .find(name)
                .ok_or_else(|| Error::Failed(alloc::format!("Unknown command `{name}`")))?;
            _ = writeln!(ctx.out, "{}", command.summary);
            ctx.out.write_colored(theme::MUTED, format_args!("usage: "));
            ctx.out.write_bold(
                theme::INFO,
                format_args!("{} {}\n", command.name, command.usage),
            );
            Ok(())
        }
        _ => Err(Error::Usage),
    }
}

fn cls(ctx: &mut Context<'_>, _: &[&str]) -> Result<(), Error> {
    ctx.out.clear();
    Ok(())
}

/// Displays the arguments separated by a single space, like the `echo` command
/// of a Unix shell.
fn echo(ctx: &mut Context<'_>, args: &[&str]) -> Result<(), Error> {
    for (i, word) in args.iter().enumerate() {
        let separator = if i == 0 { "" } else { " " };
        _ = write!(ctx.out, "{separator}{word}");
    }
    _ = writeln!(ctx.out);
    Ok(())
}

/// Displays the command lines previously executed, numbered from the oldest to
/// the newest.
fn history(ctx: &mut Context<'_>, _: &[&str]) -> Result<(), Error> {
    for (i, line) in ctx.history.iter().enumerate() {
        _ = writeln!(ctx.out, "{:>4}  {line}", i + 1);
    }
    Ok(())
}

/// Displays the usage of the physical memory and of the kernel heap, with the
/// usage ratio colored according to its level.
fn mem(ctx: &mut Context<'_>, _: &[&str]) -> Result<(), Error> {
    let pools = [
        ("Physical memory", ctx.system.physical_memory()),
        ("Kernel heap", ctx.system.heap_memory()),
    ];

    ctx.out.write_colored(
        theme::INFO,
        format_args!(
            "{:<16} {:>11} {:>11} {:>11} {:>7}\n",
            "Pool", "Total", "Used", "Free", "Usage"
        ),
    );
    for (name, usage) in pools {
        _ = write!(
            ctx.out,
            "{name:<16} {:>11} {:>11} {:>11} ",
            Bytes(usage.total),
            Bytes(usage.used),
            Bytes(usage.free()),
        );
        ctx.out.write_colored(
            theme::usage(usage.permille()),
            format_args!("{:>7}\n", Percent(usage.permille())),
        );
    }
    Ok(())
}

/// Displays the load of each CPU during the last refresh period of the console,
/// and the average load of all the CPUs.
fn cpu(ctx: &mut Context<'_>, _: &[&str]) -> Result<(), Error> {
    let stats = ctx.stats;
    ctx.out
        .write_colored(theme::INFO, format_args!("{:<6} {:>7}\n", "CPU", "Load"));
    for cpu in &stats.cpus {
        _ = write!(ctx.out, "{:<6} ", cpu.id);
        ctx.out.write_colored(
            theme::usage(cpu.permille),
            format_args!("{:>7}\n", Percent(cpu.permille)),
        );
    }
    _ = writeln!(
        ctx.out,
        "Average load: {}, {} thread(s)",
        Percent(stats.total_load()),
        stats.threads
    );
    Ok(())
}

/// Displays the time elapsed since the boot of the system, both in a human
/// readable format and in seconds.
fn uptime(ctx: &mut Context<'_>, _: &[&str]) -> Result<(), Error> {
    let uptime = ctx.system.uptime();
    _ = writeln!(
        ctx.out,
        "Up since {} ({}.{:03} s)",
        Uptime(uptime),
        uptime.as_secs(),
        uptime.subsec_millis()
    );
    Ok(())
}

fn ver(ctx: &mut Context<'_>, _: &[&str]) -> Result<(), Error> {
    _ = writeln!(ctx.out, "{}", ctx.system.version());
    Ok(())
}
