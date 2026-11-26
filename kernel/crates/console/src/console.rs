//! The console itself: it ties together the screen, the line editor, the
//! commands and the system monitor panel.
//!
//! The screen is divided into four areas, from top to bottom:
//! - a title bar, with the uptime of the system;
//! - the system monitor panel, a stack of [`Widget`]s inside a frame;
//! - the console area, with the output of the commands and the prompt;
//! - a status bar, with the main keyboard shortcuts.
use core::{fmt::Write, future::poll_fn, task::Poll};

use alloc::{boxed::Box, vec::Vec};

use crate::{
    builtins,
    color::Color,
    command::{self, Command, Context, Error, Registry},
    editor::{History, LineEditor},
    format::Uptime,
    input::{Input, Key, KeyEvent, Ticker},
    screen::{BorderStyle, Canvas, Cell, Display, Rect, Screen},
    scrollback::Scrollback,
    system::{Monitor, System},
    widget::{CpuWidget, MemoryWidget, Widget},
};

/// The prompt displayed before the command line.
const PROMPT: &str = "C:\\KIWI>";

/// The maximum number of characters of a command line.
const MAX_LINE_LENGTH: usize = 256;

/// The number of command lines remembered by the history.
const HISTORY_CAPACITY: usize = 64;

/// The number of output lines that can be scrolled back.
const SCROLLBACK_CAPACITY: usize = 512;

/// The maximum number of key presses processed before redrawing the screen.
const MAX_KEYS_PER_FRAME: usize = 32;

const TITLE_FG: Color = Color::Black;
const TITLE_BG: Color = Color::LightGray;
const PANEL_FG: Color = Color::White;
const PANEL_BG: Color = Color::Blue;
const PANEL_BORDER: Color = Color::LightCyan;
const STATUS_FG: Color = Color::Black;
const STATUS_BG: Color = Color::Cyan;
const INPUT_FG: Color = Color::White;

/// A MS-DOS like console, generic over the system it displays information
/// about.
pub struct Console<S> {
    system: S,
    screen: Screen,
    width: u16,
    height: u16,

    output: Scrollback,
    editor: LineEditor,
    history: History,
    commands: Registry,
    widgets: Vec<Box<dyn Widget>>,
    monitor: Monitor,

    /// The number of rows the console area is scrolled back by.
    scroll: usize,

    /// Whether the blinking cursor is currently visible.
    cursor_visible: bool,
}

impl<S: System> Console<S> {
    /// Creates a new console of the given size, in cells, with the builtin
    /// commands and the default widgets (memory and CPU usage).
    #[must_use]
    pub fn new(system: S, width: u16, height: u16) -> Self {
        let mut commands = Registry::new();
        builtins::register(&mut commands);

        let mut console = Self {
            screen: Screen::new(width, height),
            width,
            height,
            output: Scrollback::new(width, SCROLLBACK_CAPACITY),
            editor: LineEditor::new(MAX_LINE_LENGTH),
            history: History::new(HISTORY_CAPACITY),
            commands,
            widgets: Vec::new(),
            monitor: Monitor::new(),
            scroll: 0,
            cursor_visible: true,
            system,
        };
        console.add_widget(Box::new(MemoryWidget));
        console.add_widget(Box::new(CpuWidget));

        _ = writeln!(console.output, "{}", console.system.version());
        _ = writeln!(console.output, "Type HELP to list the available commands.");
        _ = writeln!(console.output);
        console.monitor.refresh(&console.system);
        console
    }

    /// Adds a command to the console, replacing the command with the same
    /// name if there is one.
    pub fn register(&mut self, command: Command) {
        self.commands.register(command);
    }

    /// Adds a widget at the bottom of the system monitor panel.
    pub fn add_widget(&mut self, widget: Box<dyn Widget>) {
        self.widgets.push(widget);
    }

    /// Returns the output of the console, to write messages into it.
    pub fn output(&mut self) -> &mut Scrollback {
        &mut self.output
    }

    /// Runs the console forever: waits for key presses and ticks, handles
    /// them and redraws the screen.
    pub async fn run<D, I, T>(mut self, mut display: D, mut input: I, mut ticker: T) -> !
    where
        D: Display,
        I: Input,
        T: Ticker,
    {
        let mut keys = Vec::with_capacity(MAX_KEYS_PER_FRAME);
        self.draw(&mut display);

        loop {
            // Wait until at least one key is pressed or the ticker fires, and
            // collect all the keys already available so that the screen is
            // redrawn only once for a burst of key presses.
            let mut tick = false;
            poll_fn(|cx| {
                while keys.len() < MAX_KEYS_PER_FRAME {
                    match input.poll_key(cx) {
                        Poll::Ready(key) => keys.push(key),
                        Poll::Pending => break,
                    }
                }
                tick = ticker.poll_tick(cx).is_ready();
                if keys.is_empty() && !tick {
                    Poll::Pending
                } else {
                    Poll::Ready(())
                }
            })
            .await;

            for key in keys.drain(..) {
                self.handle_key(key);
            }
            if tick {
                self.tick();
            }
            self.draw(&mut display);
        }
    }

    /// Refreshes the information about the system and blinks the cursor.
    pub fn tick(&mut self) {
        self.monitor.refresh(&self.system);
        self.cursor_visible = !self.cursor_visible;
    }

    /// Handles a key press.
    pub fn handle_key(&mut self, event: KeyEvent) {
        // Keep the cursor visible while the user is typing.
        self.cursor_visible = true;

        if event.is_ctrl('l') {
            self.output.clear();
            self.scroll = 0;
            return;
        }
        if event.is_ctrl('c') {
            let line = self.editor.take();
            _ = writeln!(self.output, "{PROMPT}{line}^C");
            self.scroll = 0;
            return;
        }

        let page = usize::from(self.console_area().height.saturating_sub(1).max(1));
        match event.key {
            Key::Char(ch) if !event.modifiers.ctrl && !event.modifiers.alt => {
                self.editor.insert(ch);
                self.scroll = 0;
            }
            Key::Enter => {
                let line = self.editor.take();
                self.execute(&line);
                self.scroll = 0;
            }
            Key::Up if event.modifiers.shift => self.scroll += 1,
            Key::Down if event.modifiers.shift => self.scroll = self.scroll.saturating_sub(1),
            Key::Up => self.editor.history_previous(&self.history),
            Key::Down => self.editor.history_next(&self.history),
            Key::PageUp => self.scroll += page,
            Key::PageDown => self.scroll = self.scroll.saturating_sub(page),
            Key::Backspace => self.editor.backspace(),
            Key::Delete => self.editor.delete(),
            Key::Left => self.editor.left(),
            Key::Right => self.editor.right(),
            Key::Home => self.editor.home(),
            Key::End => self.editor.end(),
            Key::Escape => self.editor.clear(),
            Key::Tab => self.complete(),
            Key::Function(1) => {
                self.execute("help");
                self.scroll = 0;
            }
            _ => {}
        }
    }

    /// Executes a command line, as if it was typed by the user.
    pub fn execute(&mut self, line: &str) {
        _ = write!(self.output, "{PROMPT}");
        self.output
            .write_colored(INPUT_FG, format_args!("{line}\n"));
        self.history.push(line);

        let words = command::split(line);
        let Some((name, args)) = words.split_first() else {
            return;
        };

        match self.commands.find(name).copied() {
            None => {
                _ = writeln!(self.output, "Bad command or file name");
            }
            Some(command) => {
                let mut context = Context {
                    out: &mut self.output,
                    system: &self.system,
                    stats: self.monitor.stats(),
                    commands: &self.commands,
                    history: &self.history,
                };
                let result = (command.handler)(&mut context, args);
                self.output.reset_color();
                self.output.finish_line();

                match result {
                    Ok(()) => {}
                    Err(Error::Usage) => self.output.write_colored(
                        Color::Yellow,
                        format_args!(
                            "Usage: {} {}\n",
                            command.name.to_ascii_uppercase(),
                            command.usage
                        ),
                    ),
                    Err(Error::Failed(reason)) => self
                        .output
                        .write_colored(Color::LightRed, format_args!("{reason}\n")),
                }
            }
        }

        // Separate the output of each command with a blank line, like MS-DOS.
        if !self.output.is_empty() {
            self.output.put_char('\n');
        }
    }

    /// Completes the name of the command being typed.
    fn complete(&mut self) {
        let text = self.editor.text();
        if !self.editor.at_end() || text.contains(char::is_whitespace) {
            return;
        }

        let mut candidates = self.commands.starting_with(&text);
        let Some(first) = candidates.next() else {
            return;
        };

        // Compute the longest prefix shared by all the candidates.
        let mut common = first.name;
        let mut count = 1;
        for candidate in candidates {
            let shared = common
                .bytes()
                .zip(candidate.name.bytes())
                .take_while(|(a, b)| a == b)
                .count();
            common = &common[..shared];
            count += 1;
        }

        if common.len() > text.len() {
            self.editor.insert_str(&common[text.len()..]);
        }
        if count == 1 {
            self.editor.insert(' ');
        } else if common.len() <= text.len() {
            // Nothing more can be completed: list the candidates.
            _ = writeln!(self.output, "{PROMPT}{text}");
            let names: Vec<&str> = self.commands.starting_with(&text).map(|c| c.name).collect();
            _ = writeln!(self.output, "{}", names.join("  "));
            self.scroll = 0;
        }
    }

    /// Composes the screen and draws the cells that changed on the display.
    pub fn draw<D: Display + ?Sized>(&mut self, display: &mut D) {
        self.compose();
        self.screen.present(display);
    }

    /// Forces the next frame to redraw the whole screen, for example after
    /// the display was modified by someone else.
    pub fn invalidate(&mut self) {
        self.screen.invalidate();
    }

    /// Returns the height of the system monitor panel, or 0 if the screen is
    /// too small to display it.
    fn panel_height(&self) -> u16 {
        let inner = self.width.saturating_sub(4);
        let stats = self.monitor.stats();
        let content: u16 = self.widgets.iter().map(|w| w.height(stats, inner)).sum();
        let height = content + 2;

        // Keep at least a few rows for the console area.
        if self.height >= height + 2 + 5 {
            height
        } else {
            0
        }
    }

    /// Returns the area of the screen where the output and the prompt are
    /// displayed.
    fn console_area(&self) -> Rect {
        let top = 1 + self.panel_height();
        Rect::new(0, top, self.width, self.height.saturating_sub(top + 1))
    }

    fn compose(&mut self) {
        let panel_height = self.panel_height();
        let area = self.console_area();
        let stats = self.monitor.stats();
        let mut canvas = self.screen.canvas();

        // Title bar.
        let mut title = canvas.sub(Rect::new(0, 0, self.width, 1));
        title.set_colors(TITLE_FG, TITLE_BG);
        title.fill(' ');
        title.text(1, 0, "■ ");
        title.text(3, 0, self.system.version());
        title.text_right(
            0,
            &alloc::format!(
                "Uptime {}  Threads {} ",
                Uptime(stats.uptime),
                stats.threads
            ),
        );

        // System monitor panel.
        if panel_height > 0 {
            let mut panel = canvas.sub(Rect::new(0, 1, self.width, panel_height));
            panel.set_colors(PANEL_BORDER, PANEL_BG);
            panel.fill(' ');
            panel.frame(BorderStyle::Double, Some(" System Monitor "));
            panel.set_fg(PANEL_FG);

            let inner = self.width.saturating_sub(4);
            let mut y = 1;
            for widget in &self.widgets {
                let height = widget.height(stats, inner);
                widget.draw(stats, &mut panel.sub(Rect::new(2, y, inner, height)));
                y += height;
            }
        }

        // Console area.
        let mut console = canvas.sub(area);
        let scroll = draw_console_area(
            &mut console,
            &self.output,
            &self.editor,
            self.scroll,
            self.cursor_visible,
        );
        self.scroll = scroll;

        // Status bar.
        let mut status_bar = canvas.sub(Rect::new(0, self.height.saturating_sub(1), self.width, 1));
        status_bar.set_colors(STATUS_FG, STATUS_BG);
        status_bar.fill(' ');
        status_bar.text(
            1,
            0,
            "F1=Help  \u{2191}\u{2193}=History  Tab=Complete  PgUp/PgDn=Scroll  \
             Ctrl+L=Clear  Ctrl+C=Cancel",
        );
        let position = if scroll > 0 {
            status_bar.set_fg(Color::Red);
            alloc::format!("Scrolled back {scroll} ")
        } else {
            alloc::format!("{} lines ", self.output.len())
        };
        status_bar.text_right(0, &position);
    }
}

/// Draws the output and the prompt on the console area, and returns the
/// scroll offset actually used (the requested one, clamped to the content).
fn draw_console_area(
    canvas: &mut Canvas<'_>,
    output: &Scrollback,
    editor: &LineEditor,
    scroll: usize,
    cursor_visible: bool,
) -> usize {
    let width = usize::from(canvas.width().max(1));
    let height = usize::from(canvas.height());
    let blank = Cell::new(' ', Scrollback::DEFAULT_FG, Scrollback::DEFAULT_BG);

    // The prompt and the command line, wrapped on as many rows as needed.
    let prompt: Vec<Cell> = PROMPT
        .chars()
        .map(|ch| Cell::new(ch, Scrollback::DEFAULT_FG, Scrollback::DEFAULT_BG))
        .chain(
            editor
                .chars()
                .iter()
                .map(|&ch| Cell::new(ch, INPUT_FG, Scrollback::DEFAULT_BG)),
        )
        .collect();
    let cursor = PROMPT.chars().count() + editor.cursor();
    let prompt_rows = prompt.len() / width + 1;

    let total = output.len() + prompt_rows;
    let scroll = scroll.min(total.saturating_sub(height));
    let first = total.saturating_sub(height + scroll);

    for row in 0..height {
        let index = first + row;
        let cells: &[Cell] = if let Some(line) = output.line(index) {
            line
        } else if let Some(prompt_row) = index.checked_sub(output.len())
            && prompt_row < prompt_rows
        {
            let start = (prompt_row * width).min(prompt.len());
            let end = (start + width).min(prompt.len());
            &prompt[start..end]
        } else {
            &[]
        };

        // Both values are bounded by the size of the canvas.
        #[allow(clippy::cast_possible_truncation)]
        let y = row as u16;
        for column in 0..width {
            #[allow(clippy::cast_possible_truncation)]
            let x = column as u16;
            canvas.put_cell(x, y, cells.get(column).copied().unwrap_or(blank));
        }

        let prompt_row = index.checked_sub(output.len());
        if cursor_visible && prompt_row == Some(cursor / width) {
            let column = cursor % width;
            let cell = cells.get(column).copied().unwrap_or(blank);
            // Draw a block cursor, like the VGA text mode.
            #[allow(clippy::cast_possible_truncation)]
            canvas.put_cell(
                column as u16,
                y,
                Cell::new(cell.ch, Color::Black, Color::LightGray),
            );
        }
    }
    scroll
}

impl<S> core::fmt::Debug for Console<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Console")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("scroll", &self.scroll)
            .finish_non_exhaustive()
    }
}
