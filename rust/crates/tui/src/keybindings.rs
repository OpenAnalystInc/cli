//! Keybinding dispatch for the TUI.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;
use crate::panels::chat::ChatMessage;

/// Handle a key event and dispatch to the appropriate handler.
pub fn handle_key(key: KeyEvent, app: &mut App) {
    // Permission dialog takes priority (modal)
    if app.permission_dialog.is_some() {
        handle_permission_dialog_key(key, app);
        return;
    }

    match key.code {
        // Ctrl+C → cancel running agent OR quit
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.request_exit();
        }
        // Ctrl+Shift+B → send prompt to run in background
        KeyCode::Char('B') if key.modifiers.contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT) => {
            let text = app.input_state.text();
            if !text.trim().is_empty() {
                app.input_state.clear();
                app.run_in_background(text);
            }
        }
        // Ctrl+B → toggle sidebar
        KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_sidebar();
        }
        // Ctrl+L → clear chat
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.chat.messages.clear();
            app.chat.scroll_offset = 0;
            app.chat.focused_message = None;
        }
        // Tab → cycle focus between panels
        KeyCode::Tab if !app.scroll_mode => {
            app.cycle_focus();
        }
        // Esc → toggle scroll mode
        KeyCode::Esc => {
            if app.scroll_mode {
                app.scroll_mode = false;
                app.focus = events::PanelId::Input;
                app.chat.focused_message = None;
            } else {
                app.scroll_mode = true;
                app.focus = events::PanelId::Chat;
                // Focus the last message
                if !app.chat.messages.is_empty() {
                    app.chat.focused_message = Some(app.chat.messages.len() - 1);
                }
            }
        }
        // In scroll mode: vim-like navigation
        _ if app.scroll_mode => {
            handle_scroll_mode_key(key, app);
        }
        // Page up/down always work for scrolling
        KeyCode::PageUp => {
            app.chat.scroll_up(10);
        }
        KeyCode::PageDown => {
            app.chat.scroll_down(10);
            app.chat.auto_scroll = true;
        }
        // Everything else → delegate to input box
        _ => {
            if let Some(submitted) = app.input_state.handle_key(key) {
                app.submit_prompt(submitted);
            }
        }
    }
}

fn handle_scroll_mode_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.chat.scroll_down(1);
            // Move focused message down
            if let Some(idx) = app.chat.focused_message {
                if idx + 1 < app.chat.messages.len() {
                    app.chat.focused_message = Some(idx + 1);
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.chat.scroll_up(1);
            // Move focused message up
            if let Some(idx) = app.chat.focused_message {
                if idx > 0 {
                    app.chat.focused_message = Some(idx - 1);
                }
            }
        }
        KeyCode::Char('G') => {
            app.chat.scroll_to_bottom();
            app.chat.auto_scroll = true;
            if !app.chat.messages.is_empty() {
                app.chat.focused_message = Some(app.chat.messages.len() - 1);
            }
        }
        KeyCode::Char('g') => {
            app.chat.scroll_offset = 0;
            app.chat.auto_scroll = false;
            app.chat.focused_message = Some(0);
        }
        // Enter → expand/collapse tool card under focus
        KeyCode::Enter => {
            if let Some(idx) = app.chat.focused_message {
                if let Some(ChatMessage::ToolCall { card }) = app.chat.messages.get_mut(idx) {
                    card.toggle_expand();
                }
            }
        }
        // / → jump to input and start typing a slash command
        KeyCode::Char('/') => {
            app.scroll_mode = false;
            app.focus = events::PanelId::Input;
            app.chat.focused_message = None;
            // Inject '/' into the input
            let slash_key = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
            app.input_state.handle_key(slash_key);
        }
        KeyCode::Char('i') | KeyCode::Esc => {
            app.scroll_mode = false;
            app.focus = events::PanelId::Input;
            app.chat.focused_message = None;
        }
        _ => {}
    }
}

fn handle_permission_dialog_key(key: KeyEvent, app: &mut App) {
    if let Some(ref mut dialog) = app.permission_dialog {
        match key.code {
            KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
                dialog.toggle_selection();
            }
            KeyCode::Enter => {
                let allow = dialog.is_allow_selected();
                let request_id = dialog.request_id.clone();
                app.resolve_permission(request_id, allow);
                app.permission_dialog = None;
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let request_id = dialog.request_id.clone();
                app.resolve_permission(request_id, true);
                app.permission_dialog = None;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                let request_id = dialog.request_id.clone();
                app.resolve_permission(request_id, false);
                app.permission_dialog = None;
            }
            _ => {}
        }
    }
}
