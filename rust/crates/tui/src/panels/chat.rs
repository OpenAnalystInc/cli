//! Chat panel — scrollable message list with tool cards and focus tracking.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Paragraph, Widget, Wrap};

use events::DiffLine;
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
    /// Styled banner (preserves colors from banner widget).
    Banner { lines: Vec<Line<'static>> },
    /// Inline status at end of response (time + tokens, like Claude Code).
    InlineStatus { text: String, is_error: bool },
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
    /// Scroll offset in lines — u32 for unlimited downward scrolling.
    pub scroll_offset: u32,
    pub auto_scroll: bool,
    /// Index of the currently focused message (for scroll mode navigation).
    pub focused_message: Option<usize>,
}

impl Default for ChatPanel {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
            focused_message: None,
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

    /// Add a system notice. Prevents duplicate consecutive messages.
    pub fn push_system(&mut self, text: String) {
        // Don't repeat the exact same system message consecutively
        if let Some(ChatMessage::System { text: prev }) = self.messages.last() {
            if *prev == text {
                return;
            }
        }
        self.messages.push(ChatMessage::System { text });
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Add a styled banner (preserves colors).
    pub fn push_banner(&mut self, lines: Vec<Line<'static>>) {
        self.messages.push(ChatMessage::Banner { lines });
        if self.auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Add an inline status line at the end of the last response (like Claude's "※ Done (1m · ↓ 500 tokens)").
    /// Prevents duplicate consecutive inline statuses.
    pub fn push_inline_status(&mut self, text: String, is_error: bool) {
        // Don't stack identical inline statuses
        if let Some(ChatMessage::InlineStatus { text: prev, .. }) = self.messages.last() {
            if *prev == text {
                return;
            }
        }
        self.messages.push(ChatMessage::InlineStatus { text, is_error });
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

    /// Scroll to the bottom (unlimited).
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = u32::MAX;
    }

    /// Scroll up by `n` lines.
    pub fn scroll_up(&mut self, n: u32) {
        self.auto_scroll = false;
        self.scroll_offset = self.scroll_offset.saturating_sub(n);
    }

    /// Scroll down by `n` lines.
    pub fn scroll_down(&mut self, n: u32) {
        self.scroll_offset = self.scroll_offset.saturating_add(n);
    }

    /// Render the chat panel with focus-aware border.
    pub fn render_with_focus(&self, area: Rect, buf: &mut Buffer, has_focus: bool) {
        use ratatui::widgets::{Block, Borders, BorderType};

        if has_focus {
            // Draw a thin left border to indicate focus
            let block = Block::default()
                .borders(Borders::LEFT)
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(Color::Yellow))
                .title(Line::from(vec![
                    Span::styled(" SCROLL ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                    Span::styled("j/k:nav  Enter:expand  Esc:back ", Style::default().fg(Color::Indexed(240))),
                ]));
            let inner = block.inner(area);
            block.render(area, buf);
            self.render(inner, buf);
        } else {
            self.render(area, buf);
        }
    }

    /// Render the chat panel.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let mut all_lines: Vec<Line<'_>> = Vec::new();

        for (msg_idx, msg) in self.messages.iter().enumerate() {
            let is_focused = self.focused_message == Some(msg_idx);

            match msg {
                ChatMessage::User { text } => {
                    all_lines.push(Line::from(""));
                    let bg = if is_focused { Color::Indexed(236) } else { Color::Reset };
                    all_lines.push(Line::from(vec![
                        Span::styled(" ❯ ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD).bg(bg)),
                        Span::styled(text.as_str(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD).bg(bg)),
                    ]));
                }
                ChatMessage::Assistant { markdown, streaming } => {
                    all_lines.push(Line::from(""));
                    // Status dot: blinking white while streaming, solid green when done
                    let dot_color = if *streaming {
                        let blink = (std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() / 500) % 2 == 0;
                        if blink { Color::White } else { Color::Indexed(240) }
                    } else {
                        Color::Green
                    };
                    // First line gets status dot, all lines get 2-space indent
                    let text = markdown.to_text();
                    let mut first = true;
                    for line in text.lines {
                        if first {
                            let mut spans = vec![Span::styled(" ● ", Style::default().fg(dot_color))];
                            spans.extend(line.spans);
                            all_lines.push(Line::from(spans));
                            first = false;
                        } else {
                            // Continuation lines get matching 3-space indent
                            let mut spans = vec![Span::raw("   ")];
                            spans.extend(line.spans);
                            all_lines.push(Line::from(spans));
                        }
                    }
                    all_lines.push(Line::from(""));
                }
                ChatMessage::ToolCall { card } => {
                    render_tool_card_lines(card, is_focused, &mut all_lines);
                }
                ChatMessage::System { text } => {
                    let fg = if is_focused { Color::Indexed(252) } else { Color::DarkGray };
                    let is_error = text.starts_with("Error") || text.starts_with("error")
                        || text.contains("failed") || text.contains("Failed");
                    let dot_color = if is_error { Color::Red } else { Color::Indexed(245) };
                    let mut first = true;
                    for line in text.lines() {
                        if first {
                            all_lines.push(Line::from(vec![
                                Span::styled(" ● ", Style::default().fg(dot_color)),
                                Span::styled(line.to_string(), Style::default().fg(fg)),
                            ]));
                            first = false;
                        } else {
                            all_lines.push(Line::from(Span::styled(
                                format!("   {line}"),
                                Style::default().fg(fg),
                            )));
                        }
                    }
                }
                ChatMessage::Banner { lines } => {
                    all_lines.extend(lines.iter().cloned());
                }
                ChatMessage::InlineStatus { text, is_error } => {
                    let color = if *is_error { Color::Red } else { Color::Rgb(255, 165, 0) }; // orange for success
                    all_lines.push(Line::from(""));
                    all_lines.push(Line::from(vec![
                        Span::styled("※ ", Style::default().fg(color).add_modifier(Modifier::BOLD)),
                        Span::styled(text.as_str(), Style::default().fg(color)),
                    ]));
                    all_lines.push(Line::from(""));
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

        let total_lines = all_lines.len() as u32;
        let visible_height = area.height as u32;
        let max_scroll = total_lines.saturating_sub(visible_height);
        let scroll = self.scroll_offset.min(max_scroll);

        // Slice lines manually to support unlimited scroll (beyond u16::MAX).
        let start = scroll as usize;
        let end = (start + visible_height as usize).min(all_lines.len());
        let visible_lines: Vec<Line<'_>> = all_lines.into_iter().skip(start).take(end - start).collect();

        let paragraph = Paragraph::new(Text::from(visible_lines))
            .wrap(Wrap { trim: false });
        paragraph.render(area, buf);
    }
}

/// Render a tool call card as text lines with proper formatting.
/// For Edit/Write tools with diff data, renders a rich diff view with
/// green added lines, red removed lines, line numbers, and a summary.
fn render_tool_card_lines<'a>(card: &'a ToolCallCard, _is_focused: bool, lines: &mut Vec<Line<'a>>) {
    let status_icon = match &card.status {
        tui_widgets::ToolCallStatus::Running { .. } => "●",
        tui_widgets::ToolCallStatus::Completed { .. } => "●",
        tui_widgets::ToolCallStatus::Failed { .. } => "●",
    };
    let status_color = match &card.status {
        tui_widgets::ToolCallStatus::Running { .. } => {
            // Blinking white dot while running
            let blink = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() / 500) % 2 == 0;
            if blink { Color::White } else { Color::Indexed(240) }
        }
        tui_widgets::ToolCallStatus::Completed { .. } => Color::Green,
        tui_widgets::ToolCallStatus::Failed { .. } => Color::Red,
    };

    let has_diff = card.diff.is_some();

    // For edit/write tools with diff, show "Update(file_path)" style title
    let display_name = if has_diff {
        let diff = card.diff.as_ref().unwrap();
        let short_path = shorten_path(&diff.file_path);
        format!("Update({})", short_path)
    } else {
        card.tool_name.clone()
    };

    // Title line: ● Update(crates/orchestrator/src/worker.rs) — with 1-space indent
    lines.push(Line::from(vec![
        Span::styled(
            format!(" {status_icon} "),
            Style::default().fg(status_color),
        ),
        Span::styled(
            display_name,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // For diff cards, show summary and diff lines
    if let Some(ref diff) = card.diff {
        // Summary: └  Added 38 lines, removed 3 lines
        let summary = match (diff.added, diff.removed) {
            (a, 0) => format!("└  Added {a} lines"),
            (0, r) => format!("└  Removed {r} lines"),
            (a, r) => format!("└  Added {a} lines, removed {r} lines"),
        };
        lines.push(Line::from(Span::styled(
            summary,
            Style::default().fg(Color::DarkGray),
        )));

        // Render diff hunks with line numbers and colors
        if card.expanded {
            for hunk in &diff.hunks {
                let mut old_line = hunk.old_start;
                let mut new_line = hunk.new_start;

                for diff_line in &hunk.lines {
                    match diff_line {
                        DiffLine::Context(text) => {
                            let line_num = format!("{:>5}  ", new_line);
                            lines.push(Line::from(vec![
                                Span::styled(
                                    line_num,
                                    Style::default().fg(Color::DarkGray),
                                ),
                                Span::styled(
                                    format!("  {text}"),
                                    Style::default().fg(Color::Indexed(252)),
                                ),
                            ]));
                            old_line += 1;
                            new_line += 1;
                        }
                        DiffLine::Added(text) => {
                            let line_num = format!("{:>5} +", new_line);
                            lines.push(Line::from(vec![
                                Span::styled(
                                    line_num,
                                    Style::default().fg(Color::Green),
                                ),
                                Span::styled(
                                    format!("  {text}"),
                                    Style::default()
                                        .fg(Color::Green)
                                        .bg(Color::Rgb(0, 40, 0)),
                                ),
                            ]));
                            new_line += 1;
                        }
                        DiffLine::Removed(text) => {
                            let line_num = format!("{:>5} -", old_line);
                            lines.push(Line::from(vec![
                                Span::styled(
                                    line_num,
                                    Style::default().fg(Color::Red),
                                ),
                                Span::styled(
                                    format!("  {text}"),
                                    Style::default()
                                        .fg(Color::Red)
                                        .bg(Color::Rgb(40, 0, 0)),
                                ),
                            ]));
                            old_line += 1;
                        }
                    }
                }
            }
        }

        lines.push(Line::from(""));
    } else {
        // Non-diff tool card — input preview + optional raw output
        lines.push(Line::from(Span::styled(
            format!("  {}", card.input_preview),
            Style::default().fg(Color::Indexed(252)),
        )));

        if card.expanded {
            if let Some(ref output) = card.output {
                let max_lines = 20;
                let output_lines: Vec<&str> = output.lines().collect();
                for (i, line) in output_lines.iter().enumerate() {
                    if i >= max_lines {
                        lines.push(Line::from(Span::styled(
                            format!("  ... ({} more lines)", output_lines.len() - max_lines),
                            Style::default().fg(Color::DarkGray),
                        )));
                        break;
                    }
                    lines.push(Line::from(Span::styled(
                        format!("  {line}"),
                        Style::default().fg(Color::Indexed(245)),
                    )));
                }
            }
        }

        lines.push(Line::from(""));
    }
}

/// Shorten a file path for display — keep only the last 4 path segments.
fn shorten_path(path: &str) -> &str {
    let separators: &[char] = &['/', '\\'];
    let segments: Vec<&str> = path.split(separators).filter(|s| !s.is_empty()).collect();
    if segments.len() <= 4 {
        return path;
    }
    // Walk from the end to find the start of the 4th-to-last segment
    let target = &segments[segments.len() - 4..];
    let search = target.last().unwrap_or(&"");
    if let Some(pos) = path.rfind(search) {
        let mut count = 0;
        for (i, c) in path[..=pos].char_indices().rev() {
            if c == '/' || c == '\\' {
                count += 1;
                if count == 3 {
                    return &path[i + 1..];
                }
            }
        }
    }
    path
}
