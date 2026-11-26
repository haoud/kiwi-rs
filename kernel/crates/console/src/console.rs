//! The console itself: it ties together the terminal, the line editor, the
//! commands and the panels of the sidebar.
//!
//! The screen is divided into four areas:
//! - a header, with the name of the system, its uptime and its threads;
//! - the shell, with the output of the commands and the prompt;
//! - the sidebar on the right of the shell, a stack of [`Panel`]s;
//! - a footer, with the main keyboard shortcuts.
use core::{future::poll_fn, task::Poll};

use alloc::{boxed::Box, format, string::String, vec::Vec};
use ratatui::{
    Frame, Terminal,
    backend::Backend,
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
    },
};

use crate::{
    builtins,
    command::{self, Command, Context, Error, Registry},
    editor::{History, LineEditor},
    format::Uptime,
    input::{Input, Key, KeyEvent, Ticker},
    panel::{CpuPanel, MemoryPanel, Panel},
    scrollback::{Scrollback, StyledChar},
    system::{Monitor, System},
    theme,
};

/// The name displayed before the prompt.
const PROMPT_NAME: &str = "kiwi";

/// The separator between the name and the command line.
const PROMPT_SEPARATOR: &str = " › ";

/// The maximum number of characters of a command line.
const MAX_LINE_LENGTH: usize = 256;

/// The number of command lines remembered by the history.
const HISTORY_CAPACITY: usize = 64;

/// The number of output lines that can be scrolled back.
const SCROLLBACK_CAPACITY: usize = 512;

/// The maximum number of key presses processed before redrawing the screen.
const MAX_KEYS_PER_FRAME: usize = 32;

/// The width of the sidebar, in cells.
const SIDEBAR_WIDTH: u16 = 46;

/// The minimum width of the screen to display the sidebar.
const SIDEBAR_MIN_SCREEN_WIDTH: u16 = 100;

/// The areas of the screen.
struct Areas {
    header: Rect,
    shell: Rect,
    sidebar: Option<Rect>,
    footer: Rect,
}

impl Areas {
    fn new(area: Rect) -> Self {
        let [header, body, footer] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        if area.width >= SIDEBAR_MIN_SCREEN_WIDTH {
            let [shell, sidebar] =
                Layout::horizontal([Constraint::Fill(1), Constraint::Length(SIDEBAR_WIDTH)])
                    .areas(body);
            Self {
                header,
                shell,
                sidebar: Some(sidebar),
                footer,
            }
        } else {
            Self {
                header,
                shell: body,
                sidebar: None,
                footer,
            }
        }
    }

    /// Returns the area inside the border of the shell, where the output and
    /// the prompt are displayed. One column is kept free on the right for the
    /// scrollbar.
    fn shell_content(&self) -> Rect {
        self.shell.inner(Margin::new(2, 1))
    }
}

/// A modern text user interface console, generic over the system it displays
/// information about.
pub struct Console<S> {
    system: S,
    area: Rect,

    output: Scrollback,
    editor: LineEditor,
    history: History,
    commands: Registry,
    panels: Vec<Box<dyn Panel>>,
    monitor: Monitor,

    /// The number of rows the shell is scrolled back by.
    scroll: usize,

    /// Whether the blinking cursor is currently visible.
    cursor_visible: bool,
}

impl<S: System> Console<S> {
    /// Creates a new console for a screen of the given size, in cells, with
    /// the builtin commands and the default panels (CPU and memory usage).
    #[must_use]
    pub fn new(system: S, width: u16, height: u16) -> Self {
        let mut commands = Registry::new();
        builtins::register(&mut commands);

        let area = Rect::new(0, 0, width, height);
        let content = Areas::new(area).shell_content();
        let mut console = Self {
            area,
            output: Scrollback::new(content.width, SCROLLBACK_CAPACITY),
            editor: LineEditor::new(MAX_LINE_LENGTH),
            history: History::new(HISTORY_CAPACITY),
            commands,
            panels: Vec::new(),
            monitor: Monitor::new(),
            scroll: 0,
            cursor_visible: true,
            system,
        };
        console.add_panel(Box::new(CpuPanel));
        console.add_panel(Box::new(MemoryPanel));

        console.output.write_bold(
            theme::ACCENT,
            format_args!("{}\n", console.system.version()),
        );
        console.output.write_colored(
            theme::MUTED,
            format_args!("Type `help` to list the available commands.\n\n"),
        );
        console.monitor.refresh(&console.system);
        console
    }

    /// Adds a command to the console, replacing the command with the same
    /// name if there is one.
    pub fn register(&mut self, command: Command) {
        self.commands.register(command);
    }

    /// Adds a panel at the bottom of the sidebar.
    pub fn add_panel(&mut self, panel: Box<dyn Panel>) {
        self.panels.push(panel);
    }

    /// Returns the output of the console, to write messages into it.
    pub fn output(&mut self) -> &mut Scrollback {
        &mut self.output
    }

    /// Runs the console forever on the given backend: waits for key presses
    /// and ticks, handles them and redraws the screen. Ratatui only sends to
    /// the backend the cells that changed since the previous frame.
    pub async fn run<B, I, T>(mut self, backend: B, mut input: I, mut ticker: T) -> !
    where
        B: Backend,
        I: Input,
        T: Ticker,
    {
        let mut terminal = Terminal::new(backend).ok();
        let mut keys = Vec::with_capacity(MAX_KEYS_PER_FRAME);

        loop {
            if let Some(terminal) = &mut terminal {
                _ = terminal.draw(|frame| self.render(frame));
            }

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
            self.echo_prompt(&line);
            self.output
                .write_colored(theme::ERROR, format_args!(" ^C\n"));
            self.scroll = 0;
            return;
        }

        let page = usize::from(
            Areas::new(self.area)
                .shell_content()
                .height
                .saturating_sub(1)
                .max(1),
        );
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

    /// Writes the prompt followed by a command line into the output.
    fn echo_prompt(&mut self, line: &str) {
        self.output
            .write_bold(theme::ACCENT, format_args!("{PROMPT_NAME}"));
        self.output
            .write_colored(theme::MUTED, format_args!("{PROMPT_SEPARATOR}"));
        self.output
            .write_colored(theme::TEXT, format_args!("{line}"));
    }

    /// Executes a command line, as if it was typed by the user.
    pub fn execute(&mut self, line: &str) {
        self.echo_prompt(line);
        self.output.put_char('\n');
        self.history.push(line);

        let words = command::split(line);
        let Some((name, args)) = words.split_first() else {
            return;
        };

        match self.commands.find(name).copied() {
            None => self.output.write_colored(
                theme::ERROR,
                format_args!("Unknown command `{name}`. Type `help` to list the commands.\n"),
            ),
            Some(command) => {
                let mut context = Context {
                    out: &mut self.output,
                    system: &self.system,
                    stats: self.monitor.stats(),
                    commands: &self.commands,
                    history: &self.history,
                };
                let result = (command.handler)(&mut context, args);
                self.output.reset_style();
                self.output.finish_line();

                match result {
                    Ok(()) => {}
                    Err(Error::Usage) => self.output.write_colored(
                        theme::WARNING,
                        format_args!("usage: {} {}\n", command.name, command.usage),
                    ),
                    Err(Error::Failed(reason)) => self
                        .output
                        .write_colored(theme::ERROR, format_args!("{reason}\n")),
                }
            }
        }

        // Separate the output of each command with a blank line.
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
            self.echo_prompt(&text);
            self.output.put_char('\n');
            let names: Vec<&str> = self.commands.starting_with(&text).map(|c| c.name).collect();
            self.output
                .write_colored(theme::INFO, format_args!("{}\n", names.join("  ")));
            self.scroll = 0;
        }
    }

    /// Renders the whole screen into a frame.
    pub fn render(&mut self, frame: &mut Frame<'_>) {
        let area = frame.area();
        let areas = Areas::new(area);
        let buf = frame.buffer_mut();
        Block::new().style(theme::text()).render(area, buf);

        self.render_header(areas.header, buf);
        self.render_shell(areas.shell, buf);
        if let Some(sidebar) = areas.sidebar {
            self.render_sidebar(sidebar, buf);
        }
        self.render_footer(areas.footer, buf);
    }

    fn render_header(&self, area: Rect, buf: &mut Buffer) {
        let stats = self.monitor.stats();
        Block::new()
            .style(Style::new().bg(theme::BACKGROUND_DARK))
            .render(area, buf);

        let left = Line::from(alloc::vec![
            Span::styled(
                " kiwi ",
                Style::new()
                    .fg(theme::BACKGROUND)
                    .bg(theme::ACCENT)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("  debug console", theme::muted()),
        ]);
        buf.set_line(area.x, area.y, &left, area.width);

        let right = Line::from(alloc::vec![
            Span::styled(self.system.version(), theme::muted()),
            Span::styled("  ·  up ", theme::muted()),
            Span::styled(
                format!("{}", Uptime(stats.uptime)),
                Style::new().fg(theme::TEXT)
            ),
            Span::styled("  ·  ", theme::muted()),
            Span::styled(format!("{}", stats.threads), Style::new().fg(theme::TEXT)),
            Span::styled(" threads ", theme::muted()),
        ]);
        let width = u16::try_from(right.width())
            .unwrap_or(u16::MAX)
            .min(area.width);
        buf.set_line(area.right() - width, area.y, &right, width);
    }

    fn render_shell(&mut self, area: Rect, buf: &mut Buffer) {
        let content = area.inner(Margin::new(2, 1));
        let (total, scroll) = self.render_shell_content(content, buf);
        self.scroll = scroll;

        let mut block = panel_block("shell");
        if scroll > 0 {
            block = block.title_top(
                Line::from(Span::styled(
                    format!(" scrolled back {scroll} lines "),
                    Style::new().fg(theme::WARNING),
                ))
                .right_aligned(),
            );
        }
        block.render(area, buf);

        // Display a scrollbar on the right border when the content does not
        // fit in the shell.
        let height = usize::from(content.height);
        if total > height {
            let max_scroll = total - height;
            let mut state = ScrollbarState::new(max_scroll + 1).position(max_scroll - scroll);
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .track_symbol(Some("│"))
                .track_style(Style::new().fg(theme::BORDER))
                .thumb_symbol("┃")
                .thumb_style(Style::new().fg(theme::ACCENT))
                .render(area.inner(Margin::new(0, 1)), buf, &mut state);
        }
    }

    /// Draws the output and the prompt in the given area, and returns the
    /// total number of rows of the content and the scroll offset actually
    /// used (the requested one, clamped to the content).
    fn render_shell_content(&self, area: Rect, buf: &mut Buffer) -> (usize, usize) {
        let width = usize::from(area.width.max(1));
        let height = usize::from(area.height);

        // The prompt and the command line, wrapped on as many rows as needed.
        let styled = |ch, fg, bold| StyledChar { ch, fg, bold };
        let prompt: Vec<StyledChar> = PROMPT_NAME
            .chars()
            .map(|ch| styled(ch, theme::ACCENT, true))
            .chain(
                PROMPT_SEPARATOR
                    .chars()
                    .map(|ch| styled(ch, theme::MUTED, false)),
            )
            .chain(
                self.editor
                    .chars()
                    .iter()
                    .map(|&ch| styled(ch, theme::TEXT, false)),
            )
            .collect();
        let prompt_length = PROMPT_NAME.chars().count() + PROMPT_SEPARATOR.chars().count();
        let cursor = prompt_length + self.editor.cursor();
        let prompt_rows = prompt.len() / width + 1;

        let total = self.output.len() + prompt_rows;
        let scroll = self.scroll.min(total.saturating_sub(height));
        let first = total.saturating_sub(height + scroll);

        for (row, y) in (area.top()..area.bottom()).enumerate() {
            let index = first + row;
            let cells: &[StyledChar] = if let Some(line) = self.output.line(index) {
                line
            } else if let Some(prompt_row) = index.checked_sub(self.output.len())
                && prompt_row < prompt_rows
            {
                let start = (prompt_row * width).min(prompt.len());
                let end = (start + width).min(prompt.len());
                &prompt[start..end]
            } else {
                &[]
            };

            for (cell, x) in cells.iter().zip(area.left()..area.right()) {
                let mut style = Style::new().fg(cell.fg);
                if cell.bold {
                    style = style.add_modifier(Modifier::BOLD);
                }
                buf[(x, y)].set_char(cell.ch).set_style(style);
            }

            // Draw a block cursor.
            let prompt_row = index.checked_sub(self.output.len());
            if self.cursor_visible && prompt_row == Some(cursor / width) {
                let column = u16::try_from(cursor % width).unwrap_or(0);
                buf[(area.x + column, y)]
                    .set_style(Style::new().fg(theme::BACKGROUND).bg(theme::TEXT));
            }
        }
        (total, scroll)
    }

    fn render_sidebar(&self, area: Rect, buf: &mut Buffer) {
        let stats = self.monitor.stats();
        let mut y = area.y;
        for panel in &self.panels {
            let inner_width = area.width.saturating_sub(4);
            let height = panel.height(stats, inner_width) + 2;
            let rect = Rect::new(area.x, y, area.width, height).intersection(area);
            if rect.height < 3 {
                break;
            }

            panel_block(panel.title()).render(rect, buf);
            panel.render(stats, rect.inner(Margin::new(2, 1)), buf);
            y += height;
        }
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        Block::new()
            .style(Style::new().bg(theme::BACKGROUND_DARK))
            .render(area, buf);

        let key = |text: &'static str| {
            Span::styled(
                text,
                Style::new().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
            )
        };
        let label = |text: &'static str| Span::styled(text, theme::muted());
        let hints = Line::from(alloc::vec![
            Span::raw(" "),
            key("F1"),
            label(" help   "),
            key("↑↓"),
            label(" history   "),
            key("Tab"),
            label(" complete   "),
            key("PgUp/PgDn"),
            label(" scroll   "),
            key("^L"),
            label(" clear   "),
            key("^C"),
            label(" cancel"),
        ]);
        buf.set_line(area.x, area.y, &hints, area.width);

        let lines: String = format!("{} lines ", self.output.len());
        let width = u16::try_from(lines.len())
            .unwrap_or(u16::MAX)
            .min(area.width);
        buf.set_string(area.right() - width, area.y, &lines, theme::muted());
    }
}

/// Returns the block surrounding a panel: rounded borders and a title.
fn panel_block(title: &str) -> Block<'_> {
    Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(theme::BORDER))
        .title(Line::from(Span::styled(
            format!(" {title} "),
            theme::title(),
        )))
}

impl<S> core::fmt::Debug for Console<S> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Console")
            .field("area", &self.area)
            .field("scroll", &self.scroll)
            .finish_non_exhaustive()
    }
}
