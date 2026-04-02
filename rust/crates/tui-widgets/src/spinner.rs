//! Spinner widget wrapping `throbber-widgets-tui`.
//!
//! Provides an animated braille-style spinner with our color theme.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Paragraph, Widget};
use throbber_widgets_tui::{Throbber, ThrobberState};

/// Animated spinner for loading states.
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

/// Re-export `ThrobberState` so users don't need to depend on `throbber-widgets-tui` directly.
pub use throbber_widgets_tui::ThrobberState as SpinnerState;
