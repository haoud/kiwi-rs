//! The commands that can be executed by the console.
use alloc::{string::String, vec::Vec};

use crate::{
    editor::History,
    scrollback::Scrollback,
    system::{Stats, System},
};

/// The error returned by a command that failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The arguments of the command are invalid: the usage of the command is
    /// displayed.
    Usage,

    /// The command failed for the given reason.
    Failed(String),
}

/// The function executing a command. It receives the arguments of the command,
/// without the name of the command itself.
pub type Handler = fn(&mut Context<'_>, &[&str]) -> Result<(), Error>;

/// A command that can be executed by the console.
#[derive(Debug, Clone, Copy)]
pub struct Command {
    /// The name of the command. Names are case-insensitive, like in MS-DOS.
    pub name: &'static str,

    /// A one-line description of the command, displayed by `help`.
    pub summary: &'static str,

    /// The syntax of the arguments of the command (e.g. `<count> [seconds]`),
    /// displayed by `help <command>` and when the arguments are invalid.
    pub usage: &'static str,

    /// The function executing the command.
    pub handler: Handler,
}

/// Everything a command has access to while it is executed.
pub struct Context<'a> {
    /// The output of the console, where the command writes its results.
    pub out: &'a mut Scrollback,

    /// The system the console runs on.
    pub system: &'a dyn System,

    /// The statistics of the system at the last refresh of the console.
    pub stats: &'a Stats,

    /// The commands known by the console.
    pub commands: &'a Registry,

    /// The command lines previously executed.
    pub history: &'a History,
}

/// The set of commands known by the console, sorted by name.
#[derive(Debug, Clone, Default)]
pub struct Registry {
    /// The commands, sorted by name (ignoring the case) so that a command can
    /// be found with a binary search.
    commands: Vec<Command>,
}

impl Registry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a command to the registry. If a command with the same name is
    /// already registered, it is replaced.
    pub fn register(&mut self, command: Command) {
        match self.position(command.name) {
            Ok(index) => self.commands[index] = command,
            Err(index) => self.commands.insert(index, command),
        }
    }

    /// Finds a command by its name, ignoring the case.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&Command> {
        self.position(name).ok().map(|index| &self.commands[index])
    }

    /// Returns an iterator over the commands, sorted by name.
    pub fn iter(&self) -> impl Iterator<Item = &Command> {
        self.commands.iter()
    }

    /// Returns an iterator over the commands whose name starts with the given
    /// prefix, ignoring the case.
    pub fn starting_with<'a>(&'a self, prefix: &'a str) -> impl Iterator<Item = &'a Command> {
        self.commands.iter().filter(move |command| {
            command.name.len() >= prefix.len()
                && command.name.as_bytes()[..prefix.len()].eq_ignore_ascii_case(prefix.as_bytes())
        })
    }

    /// Finds the position of a command in the sorted list of commands, ignoring
    /// the case. Returns `Ok` with the index of the command if it is
    /// registered, or `Err` with the index where it should be inserted to keep
    /// the list sorted, like `binary_search`.
    fn position(&self, name: &str) -> Result<usize, usize> {
        self.commands
            .binary_search_by(|command| compare_ignore_case(command.name, name))
    }
}

/// Compares two strings, ignoring the case of ASCII characters.
fn compare_ignore_case(a: &str, b: &str) -> core::cmp::Ordering {
    a.bytes()
        .map(|c| c.to_ascii_lowercase())
        .cmp(b.bytes().map(|c| c.to_ascii_lowercase()))
}

/// Splits a command line into words separated by whitespace. A word can be
/// enclosed in double quotes to contain whitespace: the quotes are not part of
/// the word. An unterminated quote extends to the end of the line.
#[must_use]
pub fn split(line: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut rest = line.trim_start();

    while !rest.is_empty() {
        let (word, remaining) = if let Some(quoted) = rest.strip_prefix('"') {
            quoted.split_once('"').unwrap_or((quoted, ""))
        } else {
            rest.split_once(char::is_whitespace).unwrap_or((rest, ""))
        };
        words.push(word);
        rest = remaining.trim_start();
    }
    words
}

/// Parses an integer argument, returning [`Error::Usage`] if it is invalid.
///
/// # Errors
/// Returns [`Error::Usage`] if the argument is not a valid integer.
pub fn parse<T: core::str::FromStr>(argument: &str) -> Result<T, Error> {
    argument.parse().map_err(|_| Error::Usage)
}
