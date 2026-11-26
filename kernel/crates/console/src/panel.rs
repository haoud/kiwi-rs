//! The panels of the sidebar.
//!
//! The sidebar on the right of the shell is a vertical stack of panels, each
//! one displaying some information about the system inside a bordered block.
//! New panels can be added to the console by implementing the [`Panel`] trait.
use alloc::{format, vec::Vec};

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, LineGauge, RenderDirection, Sparkline, Widget},
};

use crate::{
    format::{Bytes, Percent},
    system::{MemoryUsage, Stats},
    theme,
};

/// A panel of the sidebar.
pub trait Panel: Send {
    /// Returns the title of the panel.
    fn title(&self) -> &'static str;

    /// Returns the number of rows needed by the content of the panel (without
    /// its borders) for the given width.
    fn height(&self, stats: &Stats, width: u16) -> u16;

    /// Draws the content of the panel in the given area of the buffer. The
    /// area has the width given to [`Panel::height`] and the height returned
    /// by it, or all the remaining height of the sidebar for a flexible panel.
    fn render(&self, stats: &Stats, area: Rect, buf: &mut Buffer);

    /// Returns true if the panel takes all the height left in the sidebar
    /// below the previous panels, instead of the height it requests.
    fn is_flexible(&self) -> bool {
        false
    }

    /// Called at every tick of the console with the time elapsed since the
    /// boot, to animate the panel.
    fn update(&mut self, _uptime: core::time::Duration) {}
}

/// The height of the sparklines, in rows.
const SPARKLINE_HEIGHT: u16 = 4;

/// Draws a sparkline of the most recent samples of a history that fit in the
/// given area.
fn render_history(history: &[u64], area: Rect, color: ratatui::style::Color, buf: &mut Buffer) {
    // Fill the background of the whole area, since the sparkline only draws
    // the columns that have a sample, and draw the newest sample on the right.
    Block::new()
        .style(Style::new().bg(theme::SURFACE))
        .render(area, buf);
    let width = usize::from(area.width);
    let recent: Vec<u64> = history.iter().rev().take(width).copied().collect();
    Sparkline::default()
        .data(recent)
        .max(1000)
        .direction(RenderDirection::RightToLeft)
        .style(Style::new().fg(color).bg(theme::SURFACE))
        .render(area, buf);
}

/// Draws a thin gauge with a label on its left.
fn render_gauge(label: Line<'_>, permille: u32, area: Rect, buf: &mut Buffer) {
    LineGauge::default()
        .ratio(f64::from(permille.min(1000)) / 1000.0)
        .label(label)
        .filled_symbol("━")
        .unfilled_symbol("─")
        .filled_style(Style::new().fg(theme::usage(permille)))
        .unfilled_style(Style::new().fg(theme::BORDER))
        .render(area, buf);
}

/// Draws a line with a label on the left and a value on the right.
fn render_key_value(key: &str, value: &Line<'_>, area: Rect, buf: &mut Buffer) {
    buf.set_span(
        area.x,
        area.y,
        &Span::styled(key, theme::muted()),
        area.width,
    );
    let width = u16::try_from(value.width())
        .unwrap_or(u16::MAX)
        .min(area.width);
    buf.set_line(area.right() - width, area.y, value, width);
}

/// Displays the average load of the CPUs with its recent history, and the
/// load of each CPU.
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuPanel;

impl Panel for CpuPanel {
    fn title(&self) -> &'static str {
        "CPU"
    }

    fn height(&self, stats: &Stats, _: u16) -> u16 {
        let cpus = u16::try_from(stats.cpus.len()).unwrap_or(u16::MAX);
        1 + SPARKLINE_HEIGHT + 1 + cpus
    }

    fn render(&self, stats: &Stats, area: Rect, buf: &mut Buffer) {
        let load = stats.total_load();
        let [summary, history, _, cpus] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(SPARKLINE_HEIGHT),
            Constraint::Length(1),
            Constraint::Fill(1),
        ])
        .areas(area);

        let value = Line::from(vec_spans([
            Span::styled(
                format!("{}", Percent(load)),
                Style::new()
                    .fg(theme::usage(load))
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(format!("  {} cores", stats.cpus.len()), theme::muted()),
        ]));
        render_key_value("average load", &value, summary, buf);

        let samples: Vec<u64> = stats.load_history.iter().copied().collect();
        render_history(&samples, history, theme::TEAL, buf);

        for (row, cpu) in cpus.rows().zip(&stats.cpus) {
            let label = Line::from(vec_spans([
                Span::styled(format!("cpu{:<2} ", cpu.id), theme::muted()),
                Span::styled(
                    format!("{:>6} ", Percent(cpu.permille)),
                    Style::new().fg(theme::TEXT),
                ),
            ]));
            render_gauge(label, cpu.permille, row, buf);
        }
    }
}

/// Displays the usage of the physical memory and of the kernel heap.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryPanel;

impl MemoryPanel {
    fn render_pool(name: &str, usage: MemoryUsage, area: Rect, buf: &mut Buffer) {
        let [details, gauge] =
            Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).areas(area);
        let value = Line::from(vec_spans([
            Span::styled(
                format!("{}", Bytes(usage.used)),
                Style::new().fg(theme::TEXT),
            ),
            Span::styled(format!(" / {}", Bytes(usage.total)), theme::muted()),
        ]));
        render_key_value(name, &value, details, buf);

        let permille = usage.permille();
        let label = Span::styled(
            format!("{:>6} ", Percent(permille)),
            Style::new().fg(theme::TEXT),
        );
        render_gauge(Line::from(label), permille, gauge, buf);
    }
}

impl Panel for MemoryPanel {
    fn title(&self) -> &'static str {
        "Memory"
    }

    fn height(&self, _: &Stats, _: u16) -> u16 {
        2 + 1 + 2 + 1 + 2
    }

    fn render(&self, stats: &Stats, area: Rect, buf: &mut Buffer) {
        let [physical, _, heap, _, history] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(2),
        ])
        .areas(area);

        Self::render_pool("physical", stats.memory, physical, buf);
        Self::render_pool("kernel heap", stats.heap, heap, buf);

        let samples: Vec<u64> = stats.memory_history.iter().copied().collect();
        render_history(&samples, history, theme::INFO, buf);
    }
}

/// Builds a vector of spans from an array, to build a [`Line`].
fn vec_spans<const N: usize>(spans: [Span<'static>; N]) -> Vec<Span<'static>> {
    Vec::from(spans)
}

/// A hexadecimal dump of a memory region that scrolls continuously, looping
/// back to the start of the region when its end is reached.
#[derive(Debug, Clone, Copy)]
pub struct HexDumpPanel {
    title: &'static str,
    memory: &'static [u8],

    /// The offset of the first byte displayed when the console starts.
    start: usize,

    /// The number of lines scrolled since the console started.
    scrolled: usize,
}

impl HexDumpPanel {
    /// The time between two lines of the dump.
    pub const LINE_PERIOD: core::time::Duration = core::time::Duration::from_millis(50);

    /// The width of the address column, followed by a space.
    const ADDRESS_WIDTH: u16 = 9;

    /// Creates a panel dumping the given memory, which must never be modified
    /// since it is read without any synchronization (e.g. read-only data).
    #[must_use]
    pub const fn new(title: &'static str, memory: &'static [u8]) -> Self {
        Self {
            title,
            memory,
            start: 0,
            scrolled: 0,
        }
    }

    /// Starts the dump at the given offset in the memory, for example to skip
    /// a dull beginning.
    #[must_use]
    pub const fn starting_at(mut self, offset: usize) -> Self {
        self.start = offset;
        self
    }

    /// Returns the number of bytes displayed per line for the given width:
    /// each byte needs 3 columns in hexadecimal and 1 column in ASCII.
    fn bytes_per_line(width: u16) -> usize {
        let available = usize::from(width.saturating_sub(Self::ADDRESS_WIDTH)) / 4;
        [16, 8, 4, 2, 1]
            .into_iter()
            .find(|&n| n <= available)
            .unwrap_or(1)
    }

    /// Returns the color of a byte, depending on its kind.
    fn color(byte: u8) -> ratatui::style::Color {
        match byte {
            0 => theme::BORDER,
            b' '..=b'~' => theme::TEXT,
            0xFF => theme::WARNING,
            0x80..=0xFE => theme::TEAL,
            _ => theme::INFO,
        }
    }
}

impl Panel for HexDumpPanel {
    fn title(&self) -> &'static str {
        self.title
    }

    fn height(&self, _: &Stats, _: u16) -> u16 {
        3
    }

    fn is_flexible(&self) -> bool {
        true
    }

    fn update(&mut self, uptime: core::time::Duration) {
        let lines = uptime.as_millis() / Self::LINE_PERIOD.as_millis();
        self.scrolled = usize::try_from(lines).unwrap_or(0);
    }

    fn render(&self, _: &Stats, area: Rect, buf: &mut Buffer) {
        let per_line = Self::bytes_per_line(area.width);
        let lines = self.memory.len().div_ceil(per_line);
        if lines == 0 {
            return;
        }

        let rows = usize::from(area.height);
        let current = (self.start / per_line + self.scrolled) % lines;
        for (row, y) in (area.top()..area.bottom()).enumerate() {
            // The last row displays the current line, and the rows above it
            // the previous lines, wrapping around the region.
            let line = (current + lines * rows - (rows - 1 - row)) % lines;
            let offset = line * per_line;
            let bytes = &self.memory[offset..(offset + per_line).min(self.memory.len())];

            let current = row + 1 == rows;
            let background = if current {
                theme::SURFACE
            } else {
                theme::BACKGROUND
            };
            buf.set_style(
                Rect::new(area.x, y, area.width, 1),
                Style::new().bg(background),
            );

            let address = bytes.as_ptr() as usize;
            let address_style = if current {
                Style::new().fg(theme::ACCENT)
            } else {
                theme::muted()
            };
            // Only display the low 32 bits of the address, which are enough
            // to follow the progression of the dump in a narrow panel.
            #[allow(clippy::cast_possible_truncation)]
            buf.set_string(area.x, y, format!("{:08x}", address as u32), address_style);

            let mut x = area.x + Self::ADDRESS_WIDTH;
            let ascii_x = x + u16::try_from(per_line * 3).unwrap_or(u16::MAX);
            for (i, &byte) in bytes.iter().enumerate() {
                let style = Style::new().fg(Self::color(byte));
                buf.set_string(x, y, format!("{byte:02x}"), style);
                let ch = if byte.is_ascii_graphic() || byte == b' ' {
                    char::from(byte)
                } else {
                    '·'
                };
                let column = ascii_x + u16::try_from(i).unwrap_or(u16::MAX);
                buf.set_string(column, y, ch.encode_utf8(&mut [0; 4]), style);
                x += 3;
            }
        }
    }
}
