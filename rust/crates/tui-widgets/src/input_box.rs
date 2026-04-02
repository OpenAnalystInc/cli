//! Input box wrapping `edtui` for vim-mode text editing.
//!
//! `edtui` provides Normal/Insert/Visual modes, clipboard, undo/redo out of the box.
//! We wrap it with submit handling (Ctrl+S / Enter) and integration with our event system.

use edtui::{EditorEventHandler, EditorState, EditorTheme, EditorView, Lines};
use ratatui::buffer::Buffer;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, BorderType, Borders, Widget};

/// Wrapper around `edtui` providing a vim-mode input area with submit handling.
pub struct InputBox {
    block: Block<'static>,
}

impl Default for InputBox {
    fn default() -> Self {
        Self {
            block: Block::default()
                .borders(Borders::TOP)
                .border_type(BorderType::Plain)
                .border_style(Style::default().fg(Color::Indexed(240))),
        }
    }
}

impl InputBox {
    /// Create an input box with a custom border block.
    #[must_use]
    pub fn block(mut self, block: Block<'static>) -> Self {
        self.block = block;
        self
    }

    /// Render the input box into the given area using the provided state.
    pub fn render_with_state(self, area: Rect, buf: &mut Buffer, state: &mut InputBoxState) {
        let inner = self.block.inner(area);
        self.block.render(area, buf);

        let theme = EditorTheme::default()
            .base(Style::default().fg(Color::Indexed(252)))
            .cursor_style(Style::default().fg(Color::Black).bg(Color::White))
            .selection_style(Style::default().bg(Color::Indexed(236)));

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

    /// Handle a key event. Returns `Some(text)` if the user submitted.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<String> {
        // Ctrl+S → submit
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('s') {
            let text = self.text();
            if !text.trim().is_empty() {
                self.clear();
                return Some(text);
            }
            return None;
        }

        // Enter on single-line content → submit
        if key.code == KeyCode::Enter && !key.modifiers.contains(KeyModifiers::SHIFT) {
            let text = self.text();
            if !text.trim().is_empty() && !text.contains('\n') {
                self.clear();
                return Some(text);
            }
        }

        // Delegate to edtui vim-mode handler
        self.event_handler.on_key_event(key, &mut self.editor);
        None
    }
}
