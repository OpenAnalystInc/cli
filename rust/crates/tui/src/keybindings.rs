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

    // Autocomplete popup takes priority when active
    if app.suggestions.active {
        match key.code {
            // Tab / Down → next suggestion
            KeyCode::Tab | KeyCode::Down => {
                app.suggestions.next();
                return;
            }
            // Shift+Tab / Up → previous suggestion
            KeyCode::BackTab | KeyCode::Up => {
                app.suggestions.prev();
                return;
            }
            // Enter → accept selected suggestion
            KeyCode::Enter => {
                if let Some(cmd) = app.suggestions.accept() {
                    app.input_state.set_text(&cmd);
                    app.suggestions.dismiss();
                }
                return;
            }
            // Esc → dismiss autocomplete
            KeyCode::Esc => {
                app.suggestions.dismiss();
                return;
            }
            // Any other key → pass through (autocomplete will update below)
            _ => {}
        }
    }

    // Voice recording active — Space or Esc stops it
    if app.voice.is_recording.load(std::sync::atomic::Ordering::SeqCst) {
        if matches!(key.code, KeyCode::Char(' ') | KeyCode::Esc | KeyCode::Enter) {
            stop_voice_and_transcribe(app);
            return;
        }
        // Ignore all other keys while recording
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
        // Space (empty input, not streaming) → start voice recording
        KeyCode::Char(' ') if app.input_state.text().is_empty() && !app.is_streaming && !app.scroll_mode => {
            match app.voice.start_recording() {
                Ok(()) => {}
                Err(e) => {
                    app.chat.push_system(format!("Voice input error: {e}"));
                }
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
        // Ctrl+Up → previous history entry
        KeyCode::Up if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let current = app.input_state.text();
            if let Some(prev) = app.history.prev(&current) {
                let prev_owned = prev.to_string();
                app.input_state.set_text(&prev_owned);
            }
        }
        // Ctrl+Down → next history entry
        KeyCode::Down if key.modifiers.contains(KeyModifiers::CONTROL) => {
            if let Some(next) = app.history.next() {
                let next_owned = next.to_string();
                app.input_state.set_text(&next_owned);
            }
        }
        // Tab → cycle focus (only when autocomplete is NOT active)
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
                if !app.chat.messages.is_empty() {
                    app.chat.focused_message = Some(app.chat.messages.len() - 1);
                }
            }
        }
        // Sidebar focused: section navigation
        _ if app.focus == events::PanelId::Sidebar && app.sidebar_visible => {
            handle_sidebar_key(key, app);
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
            } else {
                // Update autocomplete based on current input
                let text = app.input_state.text();
                app.suggestions.update(&text);
                // Reset history cursor when user types
                app.history.reset_cursor();
            }
        }
    }
}

fn handle_scroll_mode_key(key: KeyEvent, app: &mut App) {
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => {
            app.chat.scroll_down(1);
            if let Some(idx) = app.chat.focused_message {
                if idx + 1 < app.chat.messages.len() {
                    app.chat.focused_message = Some(idx + 1);
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.chat.scroll_up(1);
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
        KeyCode::Enter => {
            if let Some(idx) = app.chat.focused_message {
                if let Some(ChatMessage::ToolCall { card }) = app.chat.messages.get_mut(idx) {
                    card.toggle_expand();
                }
            }
        }
        KeyCode::Char('/') => {
            app.scroll_mode = false;
            app.focus = events::PanelId::Input;
            app.chat.focused_message = None;
            let slash_key = KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);
            app.input_state.handle_key(slash_key);
            app.suggestions.update("/");
        }
        KeyCode::Char('i') | KeyCode::Esc => {
            app.scroll_mode = false;
            app.focus = events::PanelId::Input;
            app.chat.focused_message = None;
        }
        _ => {}
    }
}

fn stop_voice_and_transcribe(app: &mut App) {
    if let Some(wav_data) = app.voice.stop_recording() {
        app.chat.push_system("Transcribing...".to_string());
        let tx = app.action_tx.clone();
        std::thread::spawn(move || {
            match crate::voice::transcribe_audio(&wav_data) {
                Ok(text) => {
                    if let Ok(rt) = tokio::runtime::Runtime::new() {
                        rt.block_on(async {
                            let _ = tx
                                .send(events::Action::SubmitPrompt {
                                    text,
                                    effort_budget: None,
                                    model_override: None,
                                })
                                .await;
                        });
                    }
                }
                Err(e) => {
                    eprintln!("[voice] Transcription failed: {e}");
                }
            }
        });
    } else {
        app.chat.push_system("No audio recorded.".to_string());
    }
}

fn handle_sidebar_key(key: KeyEvent, app: &mut App) {
    match key.code {
        // j/Down → select next item in section
        KeyCode::Char('j') | KeyCode::Down => {
            app.sidebar_state.select_next();
        }
        // k/Up → select previous item in section
        KeyCode::Char('k') | KeyCode::Up => {
            app.sidebar_state.select_prev();
        }
        // Tab → cycle to next section
        KeyCode::Tab => {
            app.sidebar_state.next_section();
        }
        // Shift+Tab → cycle to previous section
        KeyCode::BackTab => {
            app.sidebar_state.prev_section();
        }
        // Enter → expand/collapse selected item
        KeyCode::Enter => {
            app.sidebar_state.toggle_expand();
        }
        // Esc/i → return focus to input
        KeyCode::Esc | KeyCode::Char('i') => {
            app.focus = events::PanelId::Input;
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
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                let request_id = dialog.request_id.clone();
                app.resolve_permission(request_id, false);
                app.permission_dialog = None;
            }
            _ => {}
        }
    }
}
