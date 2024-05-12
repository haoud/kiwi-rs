use core::fmt::Write;

/// A simple logger that use the architecture's log implementation.
struct Logger {}

impl log::Log for Logger {
    fn enabled(&self, _metadata: &log::Metadata) -> bool {
        true
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let level = match record.level() {
                log::Level::Error => "\x1B[1m\x1b[31m[!]\x1b[0m",
                log::Level::Warn => "\x1B[1m\x1b[33m[-]\x1b[0m",
                log::Level::Info => "\x1B[1m\x1b[32m[*]\x1b[0m",
                log::Level::Debug => "\x1B[1m\x1b[34m[#]\x1b[0m",
                log::Level::Trace => "\x1B[1m\x1b[35m[~]\x1b[0m",
            };
            _ = writeln!(Logger {}, "{} {}", level, record.args());
        }
    }

    fn flush(&self) {}
}

impl core::fmt::Write for Logger {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write(s);
        Ok(())
    }
}

/// Setup the logging subsystem. All log submitted to the logging subsystem
/// will be ignored until this function is called.
#[cfg(feature = "logging")]
pub fn setup() {
    log::set_max_level(log::LevelFilter::Info);
    log::set_logger(&Logger {}).unwrap();
    log::trace!("Logger initialized");
}

/// Write a message to the log. This function is only by the internal
/// logging functions, only included if the `log` feature is enabled.
/// On most platforms, this function will write to the serial port or
/// the console.
pub fn write(message: &str) {
    crate::target::log::write(message);
}
