//! Keybinding dispatch for the TUI.

use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::App;

/// Handle a key event and dispatch to the appropriate handler.
pub fn handle_key(key: KeyEvent, app: &mut App) {
    // Permission dialog takes priority (modal)
    if app.permission_dialog.is_some() {
        handle_permission_dialog_key(key, app);
        return;
    }

    match key.code {
        // Ctrl+C → cancel current agent or quit
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        // Ctrl+L → clear chat
        KeyCode::Char('l') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.chat.messages.clear();
            app.chat.scroll_offset = 0;
        }
        // Esc → toggle scroll mode
        KeyCode::Esc => {
            app.scroll_mode = !app.scroll_mode;
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
        KeyCode::Char('j') | KeyCode::Down => app.chat.scroll_down(1),
        KeyCode::Char('k') | KeyCode::Up => app.chat.scroll_up(1),
        KeyCode::Char('G') => {
            app.chat.scroll_to_bottom();
            app.chat.auto_scroll = true;
        }
        KeyCode::Char('g') => {
            app.chat.scroll_offset = 0;
            app.chat.auto_scroll = false;
        }
        KeyCode::Char('i') | KeyCode::Esc => {
            app.scroll_mode = false;
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
