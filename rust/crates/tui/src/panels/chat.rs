//! Chat panel — scrollable message list with tool cards.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget, Wrap};

use tui_widgets::{MarkdownStream, ToolCallCard};

/// Type of file output from multimedia commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileType {
    Image,
    Audio,
    Diagram,
    Text,
    Unknown,
}

impl FileType {
    fn icon(&self) -> &str {
        match self {
            Self::Image => "[IMG]",
            Self::Audio => "[AUD]",
            Self::Diagram => "[DGM]",
            Self::Text => "[TXT]",
            Self::Unknown => "[FILE]",
        }
    }

    fn color(&self) -> Color {
        match self {
            Self::Image => Color::Magenta,
            Self::Audio => Color::Yellow,
            Self::Diagram => Color::Cyan,
            Self::Text => Color::Green,
            Self::Unknown => Color::DarkGray,
        }
    }
}

/// A single message in the chat.
#[derive(Debug, Clone)]
pub enum ChatMessage {
    /// User prompt.
    User { text: String },
    /// Assistant response (may be streaming).
    Assistant {
        markdown: MarkdownStream,
        streaming: bool,
    },
    /// Inline tool call card.
    ToolCall { card: ToolCallCard },
    /// System notice (e.g., "Agent spawned").
    System { text: String },
    /// File output from multimedia commands (/image, /speak, /diagram).
    FileOutput {
        path: String,
        file_type: FileType,
        description: String,
    },
}

/// The main chat panel state.
pub struct ChatPanel {
    pub messages: Vec<ChatMessage>,
    pub scroll_offset: u16,
    pub auto_scroll: bool,
}

impl Default for ChatPanel {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
        }
    }
}

impl ChatPanel {
    /// Add a user message.
    pub fn push_user(&mut self, text: String) {
        self.messages.push(ChatMessage::User { text });
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Start a new assistant message.
    pub fn start_assistant(&mut self) {
        self.messages.push(ChatMessage::Assistant {
            markdown: MarkdownStream::new(),
            streaming: true,
        });
    }

    /// Append a streaming delta to the current assistant message.
    pub fn push_delta(&mut self, delta: &str) {
        if let Some(ChatMessage::Assistant { markdown, .. }) = self.messages.last_mut() {
            markdown.push_delta(delta);
        }
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Mark the current assistant message as done streaming.
    pub fn finish_assistant(&mut self) {
        if let Some(ChatMessage::Assistant { streaming, .. }) = self.messages.last_mut() {
            *streaming = false;
        }
    }

    /// Add a tool call card.
    pub fn push_tool_call(&mut self, card: ToolCallCard) {
        self.messages.push(ChatMessage::ToolCall { card });
    }

    /// Add a system notice.
    pub fn push_system(&mut self, text: String) {
        self.messages.push(ChatMessage::System { text });
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Add a file output message from multimedia commands.
    pub fn push_file_output(&mut self, path: String, file_type: FileType, description: String) {
        self.messages.push(ChatMessage::FileOutput {
            path,
            file_type,
            description,
        });
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Scroll to the bottom.
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = u16::MAX; // Will be clamped during render
    }

    /// Scroll up by `n` lines.
    pub fn scroll_up(&mut self, n: u16) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    /// Scroll down by `n` lines.
    pub fn scroll_down(&mut self, n: u16) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
    }

    /// Render the chat panel.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let mut all_lines: Vec<Line<'_>> = Vec::new();

        for msg in &self.messages {
            match msg {
                ChatMessage::User { text } => {
                    all_lines.push(Line::from(""));
                    all_lines.push(Line::from(vec![
                        Span::styled("❯ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::styled(text.as_str(), Style::default().fg(Color::White)),
                    ]));
                    all_lines.push(Line::from(""));
                }
                ChatMessage::Assistant { markdown, .. } => {
                    let text = markdown.to_text();
                    all_lines.extend(text.lines);
                    all_lines.push(Line::from(""));
                }
                ChatMessage::ToolCall { card } => {
                    // Render tool card as text lines
                    let status_icon = match &card.status {
                        tui_widgets::ToolCallStatus::Running { .. } => "⠋",
                        tui_widgets::ToolCallStatus::Completed { .. } => "✓",
                        tui_widgets::ToolCallStatus::Failed { .. } => "✗",
                    };
                    let status_color = match &card.status {
                        tui_widgets::ToolCallStatus::Running { .. } => Color::Blue,
                        tui_widgets::ToolCallStatus::Completed { .. } => Color::Green,
                        tui_widgets::ToolCallStatus::Failed { .. } => Color::Red,
                    };
                    let duration = card.status.duration_label();
                    all_lines.push(Line::from(vec![
                        Span::styled("  ╭─ ", Style::default().fg(Color::Indexed(245))),
                        Span::styled(status_icon, Style::default().fg(status_color)),
                        Span::raw(" "),
                        Span::styled(&card.tool_name, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                        Span::styled(format!(" ── {duration} "), Style::default().fg(Color::Indexed(245))),
                        Span::styled("─╮", Style::default().fg(Color::Indexed(245))),
                    ]));
                    all_lines.push(Line::from(vec![
                        Span::styled("  │ ", Style::default().fg(Color::Indexed(245))),
                        Span::styled(&card.input_preview, Style::default().fg(Color::Indexed(252))),
                    ]));
                    all_lines.push(Line::from(Span::styled(
                        "  ╰──────────────────────╯",
                        Style::default().fg(Color::Indexed(245)),
                    )));
                }
                ChatMessage::System { text } => {
                    for line in text.lines() {
                        all_lines.push(Line::from(Span::styled(
                            line.to_string(),
                            Style::default().fg(Color::DarkGray),
                        )));
                    }
                }
                ChatMessage::FileOutput {
                    path,
                    file_type,
                    description,
                } => {
                    let icon = file_type.icon();
                    let color = file_type.color();
                    all_lines.push(Line::from(vec![
                        Span::styled(
                            format!("  {icon} "),
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            description.as_str(),
                            Style::default().fg(Color::Indexed(252)),
                        ),
                    ]));
                    all_lines.push(Line::from(vec![
                        Span::raw("     "),
                        Span::styled(
                            path.as_str(),
                            Style::default()
                                .fg(Color::Blue)
                                .add_modifier(Modifier::UNDERLINED),
                        ),
                    ]));
                    all_lines.push(Line::from(""));
                }
            }
        }

        let total_lines = all_lines.len() as u16;
        let visible_height = area.height;
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll = self.scroll_offset.min(max_scroll);

        let paragraph = Paragraph::new(Text::from(all_lines))
            .wrap(Wrap { trim: false })
            .scroll((scroll, 0));
        paragraph.render(area, buf);

        // Scrollbar
        if total_lines > visible_height {
            let mut scrollbar_state = ScrollbarState::new(total_lines as usize)
                .position(scroll as usize)
                .viewport_content_length(visible_height as usize);
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight);
            scrollbar.render(area, buf, &mut scrollbar_state);
        }
    }
}
