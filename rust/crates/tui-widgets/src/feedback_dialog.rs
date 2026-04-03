//! Inline feedback dialog — rendered below a KnowledgeCard.
//!
//! Shows: "Was this helpful?  [Y 👍]  [N 👎]  [Esc dismiss]"
//! Follows the PermissionDialog pattern for keybinding handling.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

/// Inline feedback prompt for a knowledge query result.
#[derive(Debug, Clone)]
pub struct FeedbackDialog {
    /// Query ID to attach feedback to.
    pub query_id: i64,
    /// 0 = thumbs up, 1 = thumbs down, 2 = dismiss
    pub selected: usize,
}

impl FeedbackDialog {
    /// Cycle to the next selection.
    pub fn toggle_selection(&mut self) {
        self.selected = (self.selected + 1) % 3;
    }

    /// Check if thumbs-up is selected.
    #[must_use]
    pub fn is_positive(&self) -> bool {
        self.selected == 0
    }

    /// Check if thumbs-down is selected.
    #[must_use]
    pub fn is_negative(&self) -> bool {
        self.selected == 1
    }

    /// Check if dismiss is selected.
    #[must_use]
    pub fn is_dismiss(&self) -> bool {
        self.selected == 2
    }
}

impl Widget for FeedbackDialog {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 20 || area.height < 1 {
            return;
        }

        let up_style = if self.selected == 0 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };

        let down_style = if self.selected == 1 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };

        let dismiss_style = if self.selected == 2 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Indexed(245))
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Indexed(240))
        };

        let line = Line::from(vec![
            Span::styled(
                "  Was this helpful? ",
                Style::default().fg(Color::Indexed(252)),
            ),
            Span::styled(" Y ", up_style),
            Span::styled(" ", Style::default()),
            Span::styled(" N ", down_style),
            Span::styled(" ", Style::default()),
            Span::styled(" Esc ", dismiss_style),
            Span::styled(
                " · /feedback for corrections",
                Style::default().fg(Color::Indexed(238)),
            ),
        ]);

        Paragraph::new(vec![line]).render(area, buf);
    }
}
