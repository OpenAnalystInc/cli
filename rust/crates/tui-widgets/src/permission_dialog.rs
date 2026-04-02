//! Modal permission dialog — shown when a tool requires elevated access.
//!
//! Custom widget (no existing crate). Renders as a centered overlay with
//! tool details and Allow/Deny buttons.

use ratatui::buffer::Buffer;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph, Widget, Wrap};

/// A modal permission dialog overlay.
#[derive(Debug, Clone)]
pub struct PermissionDialog {
    pub request_id: String,
    pub agent_id: String,
    pub tool_name: String,
    pub input_preview: String,
    pub required_mode: String,
    /// Which button is currently focused: 0 = Allow, 1 = Deny.
    pub selected: usize,
}

impl PermissionDialog {
    /// Toggle between Allow and Deny.
    pub fn toggle_selection(&mut self) {
        self.selected = if self.selected == 0 { 1 } else { 0 };
    }

    /// Returns true if Allow is selected.
    #[must_use]
    pub fn is_allow_selected(&self) -> bool {
        self.selected == 0
    }
}

impl Widget for PermissionDialog {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Center the dialog (50x14 or smaller)
        let dialog_width = area.width.min(60);
        let dialog_height = area.height.min(14);
        let x = area.x + (area.width.saturating_sub(dialog_width)) / 2;
        let y = area.y + (area.height.saturating_sub(dialog_height)) / 2;
        let dialog_area = Rect::new(x, y, dialog_width, dialog_height);

        // Clear the background
        Clear.render(dialog_area, buf);

        let block = Block::default()
            .title(Line::from(vec![
                Span::styled(" Permission Required ", Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)),
            ]))
            .title_alignment(Alignment::Center)
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Yellow))
            .padding(Padding::uniform(1));

        let inner = block.inner(dialog_area);
        block.render(dialog_area, buf);

        // Layout: info lines + button row
        let layout = Layout::vertical([
            Constraint::Min(3),    // info
            Constraint::Length(1), // spacer
            Constraint::Length(1), // buttons
        ])
        .split(inner);

        // Info text
        let info = vec![
            Line::from(vec![
                Span::styled("Tool: ", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    self.tool_name,
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
            ]),
            Line::from(vec![
                Span::styled("Requires: ", Style::default().fg(Color::DarkGray)),
                Span::styled(self.required_mode, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(""),
            Line::from(Span::styled(
                self.input_preview,
                Style::default().fg(Color::Indexed(252)),
            )),
        ];
        Paragraph::new(info)
            .wrap(Wrap { trim: true })
            .render(layout[0], buf);

        // Buttons
        let allow_style = if self.selected == 0 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green)
        };
        let deny_style = if self.selected == 1 {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Red)
        };

        let buttons = Line::from(vec![
            Span::styled("  [ Allow ]  ", allow_style),
            Span::raw("    "),
            Span::styled("  [ Deny ]  ", deny_style),
        ]);
        Paragraph::new(buttons)
            .alignment(Alignment::Center)
            .render(layout[2], buf);
    }
}
