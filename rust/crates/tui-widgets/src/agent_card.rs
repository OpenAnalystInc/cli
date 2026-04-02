//! Agent status card widget — shows spawned sub-agent info.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget};

use events::{AgentStatus, AgentType};

/// Displays the status of a spawned agent in the sidebar or agent panel.
#[derive(Debug, Clone)]
pub struct AgentStatusCard {
    pub agent_id: String,
    pub agent_type: AgentType,
    pub task_summary: String,
    pub status: AgentStatus,
}

impl Widget for AgentStatusCard {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let (icon, color) = match &self.status {
            AgentStatus::Pending => ("◦", Color::DarkGray),
            AgentStatus::Running => ("●", Color::Blue),
            AgentStatus::Completed => ("✓", Color::Green),
            AgentStatus::Failed => ("✗", Color::Red),
        };

        let title = Line::from(vec![
            Span::styled(format!("{icon} "), Style::default().fg(color)),
            Span::styled(
                self.agent_type.to_string(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Indexed(240)))
            .title(title)
            .padding(Padding::horizontal(1));

        let inner = block.inner(area);
        block.render(area, buf);

        let max_width = inner.width as usize;
        let char_count = self.task_summary.chars().count();
        let task_display = if char_count > max_width && max_width > 3 {
            let truncated: String = self.task_summary.chars().take(max_width - 3).collect();
            format!("{truncated}...")
        } else {
            self.task_summary.clone()
        };

        let paragraph = Paragraph::new(Line::from(Span::styled(
            task_display,
            Style::default().fg(Color::Indexed(252)),
        )));
        paragraph.render(inner, buf);
    }
}
