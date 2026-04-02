//! Input box wrapping `edtui` for vim-mode text editing.
//!
//! `edtui` provides Normal/Insert/Visual modes, clipboard, undo/redo out of the box.
//! We wrap it with submit handling (Ctrl+S / Enter), mode-aware borders, and
//! dynamic height calculation.

use edtui::{EditorEventHandler, EditorState, EditorTheme, EditorView, Lines};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

/// The current mode displayed in the input box title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    /// Ready for user input.
    Ready,
    /// An agent is running — show which one.
    AgentRunning { label: String },
    /// A plan is being executed.
    PlanRunning { label: String },
    /// Streaming response in progress.
    Streaming,
}

impl Default for InputMode {
    fn default() -> Self {
        Self::Ready
    }
}

impl InputMode {
    fn border_color(&self) -> Color {
        match self {
            Self::Ready => Color::Indexed(240),         // dim gray
            Self::AgentRunning { .. } => Color::Blue,   // blue for agents
            Self::PlanRunning { .. } => Color::Yellow,  // yellow for plans
            Self::Streaming => Color::Cyan,              // cyan while streaming
        }
    }

    fn title_spans(&self) -> Vec<Span<'static>> {
        match self {
            Self::Ready => vec![
                Span::styled(
                    " ❯ ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Enter to send · Shift+Enter newline ",
                    Style::default().fg(Color::Indexed(240)),
                ),
            ],
            Self::AgentRunning { label } => vec![
                Span::styled(
                    " ● ",
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{label} "),
                    Style::default().fg(Color::Blue).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "· Ctrl+C to cancel ",
                    Style::default().fg(Color::Indexed(240)),
                ),
            ],
            Self::PlanRunning { label } => vec![
                Span::styled(
                    " ◈ ",
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("{label} "),
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "· Ctrl+C to cancel ",
                    Style::default().fg(Color::Indexed(240)),
                ),
            ],
            Self::Streaming => vec![
                Span::styled(
                    " ⠋ ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "Responding... ",
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(
                    "· Ctrl+C to cancel ",
                    Style::default().fg(Color::Indexed(240)),
                ),
            ],
        }
    }
}

/// Wrapper around `edtui` providing a vim-mode input area with mode-aware styling.
pub struct InputBox {
    mode: InputMode,
}

impl Default for InputBox {
    fn default() -> Self {
        Self {
            mode: InputMode::Ready,
        }
    }
}

impl InputBox {
    /// Set the input mode (changes border color and title).
    #[must_use]
    pub fn mode(mut self, mode: InputMode) -> Self {
        self.mode = mode;
        self
    }

    /// Render the input box into the given area using the provided state.
    pub fn render_with_state(self, area: Rect, buf: &mut Buffer, state: &mut InputBoxState) {
        let border_color = self.mode.border_color();
        let title_spans = self.mode.title_spans();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(Line::from(title_spans));

        let inner = block.inner(area);
        block.render(area, buf);

        let theme = EditorTheme {
            base: Style::default().fg(Color::Indexed(252)),
            cursor_style: Style::default().fg(Color::Black).bg(Color::White),
            selection_style: Style::default().bg(Color::Indexed(236)),
            status_line: None, // No "Normal" / "Insert" label — keep it clean
            block: None,
            line_numbers_style: Style::default().fg(Color::Indexed(238)),
        };

        let editor = EditorView::new(&mut state.editor).theme(theme);
        editor.render(inner, buf);
    }
}

/// State for the input box, wrapping `edtui::EditorState` and `EditorEventHandler`.
pub struct InputBoxState {
    pub editor: EditorState,
    pub event_handler: EditorEventHandler,
}

impl Default for InputBoxState {
    fn default() -> Self {
        Self {
            editor: EditorState::default(),
            event_handler: EditorEventHandler::vim_mode(),
        }
    }
}

impl InputBoxState {
    /// Get the current text content.
    #[must_use]
    pub fn text(&self) -> String {
        String::from(self.editor.lines.clone())
    }

    /// Clear the input.
    pub fn clear(&mut self) {
        self.editor = EditorState::new(Lines::default());
    }

    /// Get the number of lines in the current editor content.
    /// Used by the layout to dynamically size the input area.
    #[must_use]
    pub fn line_count(&self) -> u16 {
        let text = self.text();
        let lines = text.lines().count().max(1) as u16;
        // +2 for border top/bottom
        lines + 2
    }

    /// Handle a key event. Returns `Some(text)` if the user submitted.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<String> {
        // Ctrl+S → submit (always)
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            let text = self.text();
            if !text.trim().is_empty() {
                self.clear();
                return Some(text);
            }
            return None;
        }

        // Enter → submit (works for single-line and multiline/pasted text)
        if key.code == KeyCode::Enter && !key.modifiers.contains(KeyModifiers::SHIFT) {
            let text = self.text();
            if !text.trim().is_empty() {
                self.clear();
                return Some(text);
            }
        }

        // Delegate to edtui vim-mode handler
        self.event_handler.on_key_event(key, &mut self.editor);
        None
    }
}
