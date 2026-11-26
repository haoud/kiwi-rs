//! The display elements of the system monitor panel.
//!
//! The panel at the top of the console is a vertical stack of widgets, each
//! one drawing some information about the system. New elements can be added
//! to the panel by implementing the [`Widget`] trait.
use alloc::format;

use crate::{
    color::Color,
    format::{Bytes, Percent},
    screen::{Canvas, Rect},
    system::{MemoryUsage, Stats},
};

/// An element of the system monitor panel.
pub trait Widget: Send {
    /// Returns the number of rows needed by the widget to draw itself on a
    /// canvas of the given width.
    fn height(&self, stats: &Stats, width: u16) -> u16;

    /// Draws the widget. The canvas has the width given to
    /// [`Widget::height`] and the height returned by it, and is already
    /// cleared with the colors of the panel.
    fn draw(&self, stats: &Stats, canvas: &mut Canvas<'_>);
}

/// The width of the labels on the left of the gauges.
const LABEL_WIDTH: u16 = 10;

/// The width of the details on the right of the gauges. It is the same for
/// all the gauges, so that they are aligned.
const DETAILS_WIDTH: u16 = 34;

/// Draws a row made of a label, a gauge filling the available width, and
/// details on the right of the gauge.
fn gauge_row(canvas: &mut Canvas<'_>, y: u16, label: &str, permille: u32, details: &str) {
    let gauge_width = canvas
        .width()
        .saturating_sub(LABEL_WIDTH + DETAILS_WIDTH + 2);

    canvas.text(0, y, label);
    canvas.put(LABEL_WIDTH, y, '[');
    canvas.gauge(
        LABEL_WIDTH + 1,
        y,
        gauge_width,
        permille,
        Color::for_usage(permille),
    );
    canvas.put(LABEL_WIDTH + 1 + gauge_width, y, ']');
    canvas.text(LABEL_WIDTH + 2 + gauge_width, y, details);
}

/// Displays the usage of the physical memory and of the kernel heap.
#[derive(Debug, Clone, Copy, Default)]
pub struct MemoryWidget;

impl MemoryWidget {
    fn draw_pool(canvas: &mut Canvas<'_>, y: u16, label: &str, usage: MemoryUsage) {
        let permille = usage.permille();
        let details = format!(
            " {:>6}  {:>10} / {:<10}",
            Percent(permille),
            Bytes(usage.used),
            Bytes(usage.total),
        );
        gauge_row(canvas, y, label, permille, &details);
    }
}

impl Widget for MemoryWidget {
    fn height(&self, _: &Stats, _: u16) -> u16 {
        2
    }

    fn draw(&self, stats: &Stats, canvas: &mut Canvas<'_>) {
        Self::draw_pool(canvas, 0, "Memory", stats.memory);
        Self::draw_pool(canvas, 1, "Heap", stats.heap);
    }
}

/// Displays the average load of the CPUs, and the load of each CPU.
#[derive(Debug, Clone, Copy, Default)]
pub struct CpuWidget;

impl CpuWidget {
    /// The width of the area used to display the load of one CPU.
    const CPU_WIDTH: u16 = 24;

    fn columns(width: u16) -> u16 {
        (width / Self::CPU_WIDTH).max(1)
    }
}

impl Widget for CpuWidget {
    fn height(&self, stats: &Stats, width: u16) -> u16 {
        let cpus = u16::try_from(stats.cpus.len()).unwrap_or(u16::MAX);
        1 + cpus.div_ceil(Self::columns(width))
    }

    fn draw(&self, stats: &Stats, canvas: &mut Canvas<'_>) {
        let load = stats.total_load();
        let summary = format!(
            " {:>6}  {} CPU(s), {} thread(s)",
            Percent(load),
            stats.cpus.len(),
            stats.threads
        );
        gauge_row(canvas, 0, "CPU", load, &summary);

        // Display the CPUs in columns, below the average load.
        let columns = Self::columns(canvas.width());
        let column_width = canvas.width() / columns;
        for (i, cpu) in stats.cpus.iter().enumerate() {
            let i = u16::try_from(i).unwrap_or(u16::MAX);
            let area = Rect::new(
                (i % columns) * column_width,
                1 + i / columns,
                column_width,
                1,
            );
            let mut cell = canvas.sub(area);
            let label = format!("#{:<3}", cpu.id);
            let percent = format!("{:>6} ", Percent(cpu.permille));
            let gauge = cell.width().saturating_sub(4 + 2 + 7 + 1);

            let x = cell.text(0, 0, &label);
            cell.put(x, 0, '[');
            cell.gauge(
                x + 1,
                0,
                gauge,
                cpu.permille,
                Color::for_usage(cpu.permille),
            );
            cell.put(x + 1 + gauge, 0, ']');
            cell.text(x + 2 + gauge, 0, &percent);
        }
    }
}
