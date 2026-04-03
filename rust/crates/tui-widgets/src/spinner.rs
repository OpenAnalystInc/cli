//! Spinner widget wrapping `throbber-widgets-tui`.
//!
//! Provides an animated braille-style spinner with brand-color cycling.
//! Inspired by Gemini CLI's `GeminiSpinner` — smooth 4-second color cycle
//! through brand colors at ~10fps (100ms tick rate).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Paragraph, Widget};
use throbber_widgets_tui::{Throbber, ThrobberState};

/// Brand color palette for the spinner cycle.
/// Cycles: Blue → Cyan → Green → Yellow → Blue (OpenAnalyst brand)
const BRAND_COLORS: &[(u8, u8, u8)] = &[
    (50, 130, 255),   // OA Blue (primary)
    (80, 160, 255),   // Light Blue
    (50, 200, 220),   // Cyan
    (80, 220, 160),   // Teal/Green
    (130, 200, 80),   // Green
    (200, 180, 50),   // Yellow
    (80, 160, 255),   // Light Blue (return)
    (50, 130, 255),   // OA Blue (wrap)
];

/// Number of ticks for one full color cycle (~4 seconds at 100ms tick).
const CYCLE_TICKS: u32 = 40;

/// Animated spinner for loading states with brand-color cycling.
pub struct OaSpinner<'a> {
    label: &'a str,
    color: Color,
    state: &'a ThrobberState,
}

impl<'a> OaSpinner<'a> {
    /// Create a spinner with a label.
    #[must_use]
    pub fn new(label: &'a str, state: &'a ThrobberState) -> Self {
        Self {
            label,
            color: Color::Blue,
            state,
        }
    }

    /// Set the spinner color.
    #[must_use]
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl Widget for OaSpinner<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let throbber = Throbber::default()
            .label(self.label)
            .style(Style::default().fg(self.color))
            .throbber_style(Style::default().fg(self.color));

        // Use to_line() to get a renderable Line without needing StatefulWidget
        let line = throbber.to_line(self.state);
        Paragraph::new(line).render(area, buf);
    }
}

/// Extended spinner state with color cycling.
pub struct SpinnerState {
    pub throbber: ThrobberState,
    tick_count: u32,
}

impl Default for SpinnerState {
    fn default() -> Self {
        Self {
            throbber: ThrobberState::default(),
            tick_count: 0,
        }
    }
}

impl SpinnerState {
    /// Advance the spinner animation (call every tick, ~100ms).
    pub fn calc_next(&mut self) {
        self.throbber.calc_next();
        self.tick_count = self.tick_count.wrapping_add(1);
    }

    /// Get the current interpolated brand color for the spinner.
    #[must_use]
    pub fn current_color(&self) -> Color {
        let t = (self.tick_count % CYCLE_TICKS) as f32 / CYCLE_TICKS as f32;
        let segment_count = BRAND_COLORS.len() - 1;
        let segment_f = t * segment_count as f32;
        let segment = (segment_f as usize).min(segment_count - 1);
        let frac = segment_f - segment as f32;

        let (r1, g1, b1) = BRAND_COLORS[segment];
        let (r2, g2, b2) = BRAND_COLORS[segment + 1];

        let r = lerp_u8(r1, r2, frac);
        let g = lerp_u8(g1, g2, frac);
        let b = lerp_u8(b1, b2, frac);

        Color::Rgb(r, g, b)
    }

    /// Get the throbber state for rendering.
    pub fn throbber(&self) -> &ThrobberState {
        &self.throbber
    }
}

/// Linear interpolation between two u8 values.
fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    let a = a as f32;
    let b = b as f32;
    (a + (b - a) * t).round().clamp(0.0, 255.0) as u8
}
