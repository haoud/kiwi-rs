//! Formatting helpers, written with integer arithmetic only since the console
//! may run in an environment without floating point support (e.g. a kernel).
use core::{fmt, time::Duration};

/// Formats a number of bytes with a binary unit and one decimal (for example
/// `12.5 MiB`).
#[derive(Debug, Clone, Copy)]
pub struct Bytes(pub u64);

impl fmt::Display for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const UNITS: [&str; 5] = ["KiB", "MiB", "GiB", "TiB", "PiB"];
        if self.0 < 1024 {
            return pad(f, format_args!("{} B", self.0));
        }

        let mut unit = 0;
        let mut divisor: u64 = 1024;
        while unit + 1 < UNITS.len() && self.0 / divisor >= 1024 {
            divisor *= 1024;
            unit += 1;
        }
        let tenths = u128::from(self.0) * 10 / u128::from(divisor);
        pad(
            f,
            format_args!("{}.{} {}", tenths / 10, tenths % 10, UNITS[unit]),
        )
    }
}

/// Formats a ratio expressed in per mille as a percentage with one decimal
/// (for example `42.7%`).
#[derive(Debug, Clone, Copy)]
pub struct Percent(pub u32);

impl fmt::Display for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        pad(f, format_args!("{}.{}%", self.0 / 10, self.0 % 10))
    }
}

/// Formats a duration as `HH:MM:SS`, prefixed by the number of days if the
/// duration is longer than a day.
#[derive(Debug, Clone, Copy)]
pub struct Uptime(pub Duration);

impl fmt::Display for Uptime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let secs = self.0.as_secs();
        let (days, hours, minutes, seconds) =
            (secs / 86400, secs / 3600 % 24, secs / 60 % 60, secs % 60);
        if days > 0 {
            pad(
                f,
                format_args!("{days}d {hours:02}:{minutes:02}:{seconds:02}"),
            )
        } else {
            pad(f, format_args!("{hours:02}:{minutes:02}:{seconds:02}"))
        }
    }
}

/// Returns `part / total` in per mille, or 0 if `total` is zero.
#[must_use]
pub fn permille(part: u64, total: u64) -> u32 {
    if total == 0 {
        return 0;
    }
    let ratio = u128::from(part.min(total)) * 1000 / u128::from(total);
    u32::try_from(ratio).unwrap_or(1000)
}

/// Writes the formatted arguments while honoring the width and alignment
/// requested by the caller (e.g. `{:>9}`), which `write!` alone does not.
fn pad(f: &mut fmt::Formatter<'_>, args: fmt::Arguments<'_>) -> fmt::Result {
    let mut buffer = Buffer::<32>::new();
    if fmt::write(&mut buffer, args).is_err() {
        return f.write_fmt(args);
    }
    f.pad(buffer.as_str())
}

/// A small fixed-size buffer implementing `fmt::Write`, used to format short
/// strings without allocating.
struct Buffer<const N: usize> {
    data: [u8; N],
    len: usize,
}

impl<const N: usize> Buffer<N> {
    fn new() -> Self {
        Self {
            data: [0; N],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        // Only complete UTF-8 strings are written into the buffer.
        core::str::from_utf8(&self.data[..self.len]).unwrap_or_default()
    }
}

impl<const N: usize> fmt::Write for Buffer<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let end = self.len + s.len();
        if end > N {
            return Err(fmt::Error);
        }
        self.data[self.len..end].copy_from_slice(s.as_bytes());
        self.len = end;
        Ok(())
    }
}
