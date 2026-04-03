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
        // Ctrl+C → cancel running agent OR quit (double-press like Claude Code)
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.request_exit();
            return; // Don't clear exit_pending below
        }
        // Ctrl+B → run in background (Claude Code parity)
        KeyCode::Char('b') | KeyCode::Char('\x02') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            let text = app.input_state.text();
            if !text.trim().is_empty() {
                app.input_state.clear();
                app.run_in_background(text);
            }
        }
        // Space (empty input, not streaming) → start voice recording
        KeyCode::Char(' ') if app.input_state.text().is_empty() && !app.is_streaming && !app.scroll_mode => {
            if !crate::voice::VoiceState::has_microphone() {
                app.chat.push_system("No microphone detected. Voice input unavailable.".to_string());
            } else {
                match app.voice.start_recording() {
                    Ok(()) => {}
                    Err(e) => {
                        app.chat.push_system(format!("Voice input error: {e}"));
                    }
                }
            }
        }
        // Ctrl+P → cycle permission mode (Default → Plan → Accept Edits → Danger)
        // Handle both 'p' and '\x10' (ASCII DLE) for Windows terminal compatibility
        KeyCode::Char('p') | KeyCode::Char('\x10') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.cycle_permission_level();
        }
        // Ctrl+\\ → toggle sidebar (unique binding, Ctrl+B is background)
        KeyCode::Char('\\') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.toggle_sidebar();
        }
        // F2 → toggle sidebar visibility/focus
        KeyCode::F(2) => {
            if app.sidebar_visible && app.focus == events::PanelId::Sidebar {
                // Already focused → hide sidebar
                app.sidebar_visible = false;
                app.focus = events::PanelId::Input;
                app.sidebar_state.has_focus = false;
            } else if app.sidebar_visible {
                // Visible but not focused → focus it
                app.focus = events::PanelId::Sidebar;
                app.sidebar_state.has_focus = true;
            } else {
                // Hidden → show and focus
                app.sidebar_visible = true;
                app.focus = events::PanelId::Sidebar;
                app.sidebar_state.has_focus = true;
            }
        }
        // Ctrl+L → clear chat
        KeyCode::Char('l') | KeyCode::Char('\x0c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.chat.messages.clear();
            app.chat.scroll_offset = 0;
            app.chat.focused_message = None;
        }
        // Up → previous history entry (when input is empty or single-line, like Claude Code)
        KeyCode::Up if !app.scroll_mode => {
            let current = app.input_state.text();
            // Only navigate history if input is empty or single-line (no newlines)
            if !current.contains('\n') {
                if let Some(prev) = app.history.prev(&current) {
                    let prev_owned = prev.to_string();
                    app.input_state.set_text(&prev_owned);
                }
            } else {
                // Multi-line: let edtui handle cursor movement
                app.input_state.event_handler.on_key_event(key, &mut app.input_state.editor);
            }
        }
        // Down → next history entry (when input is empty or single-line)
        KeyCode::Down if !app.scroll_mode => {
            let current = app.input_state.text();
            if !current.contains('\n') {
                if let Some(next) = app.history.next() {
                    let next_owned = next.to_string();
                    app.input_state.set_text(&next_owned);
                }
            } else {
                app.input_state.event_handler.on_key_event(key, &mut app.input_state.editor);
            }
        }
        // Tab → focus sidebar (from input) or cycle sections (when already in sidebar)
        KeyCode::Tab if app.sidebar_visible => {
            if app.focus == events::PanelId::Sidebar {
                app.sidebar_state.next_section();
            } else {
                // Switch focus to sidebar
                app.focus = events::PanelId::Sidebar;
                app.sidebar_state.has_focus = true;
                app.scroll_mode = false;
            }
        }
        // Shift+Tab → previous sidebar section or focus sidebar from input
        KeyCode::BackTab => {
            if app.sidebar_visible {
                if app.focus == events::PanelId::Sidebar {
                    app.sidebar_state.prev_section();
                } else {
                    // Switch focus to sidebar
                    app.focus = events::PanelId::Sidebar;
                    app.sidebar_state.has_focus = true;
                    app.scroll_mode = false;
                }
            }
        }
        // Esc → return from sidebar/scroll to input, or enter scroll mode
        KeyCode::Esc => {
            if app.focus == events::PanelId::Sidebar {
                // Return from sidebar to input
                app.focus = events::PanelId::Input;
                app.sidebar_state.has_focus = false;
            } else if app.scroll_mode {
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
        // Page up/down → ALWAYS scroll chat (from any mode)
        KeyCode::PageUp => {
            app.chat.scroll_up(10);
            app.chat.auto_scroll = false;
        }
        KeyCode::PageDown => {
            app.chat.scroll_down(10);
        }
        // Home → scroll to top
        KeyCode::Home if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.chat.scroll_offset = 0;
            app.chat.auto_scroll = false;
        }
        // End → scroll to bottom
        KeyCode::End if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.chat.scroll_to_bottom();
            app.chat.auto_scroll = true;
        }
        // Sidebar focused: section navigation
        _ if app.focus == events::PanelId::Sidebar && app.sidebar_visible => {
            handle_sidebar_key(key, app);
        }
        // In scroll mode: vim-like navigation
        _ if app.scroll_mode => {
            handle_scroll_mode_key(key, app);
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

    // Any non-Ctrl+C key resets the exit confirmation
    app.clear_exit_pending();
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
        KeyCode::PageUp => {
            app.chat.scroll_up(10);
            app.chat.auto_scroll = false;
        }
        KeyCode::PageDown => {
            app.chat.scroll_down(10);
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
        // Enter → expand/collapse, select agent, or cycle model tier
        KeyCode::Enter => {
            use crate::panels::sidebar::SidebarSection;
            use orchestrator::router::ActionCategory;
            match app.sidebar_state.active_section {
                SidebarSection::Agents => {
                    let running_count = app.sidebar_state.agents.len();
                    let idx = app.sidebar_state.selected_index;
                    if idx >= running_count {
                        let def_idx = idx - running_count;
                        let name = app.sidebar_state.toggle_agent_selection(def_idx);
                        app.set_active_agent(name);
                    } else {
                        app.sidebar_state.toggle_expand();
                    }
                }
                SidebarSection::Routing => {
                    // Cycle through available models for the selected category
                    let idx = app.sidebar_state.selected_index;
                    if let Some(cat) = ActionCategory::ALL.get(idx) {
                        if let Some(new_model) = app.sidebar_state.cycle_routing_model(idx) {
                            // Update the resolver to use this model for the category's tier
                            let profile = app.router.table.get_mut(*cat);
                            // Classify the new model to set the right tier
                            profile.model_tier = orchestrator::router::classify_model(&new_model);
                            // Update the resolver's slot for this tier
                            match profile.model_tier {
                                orchestrator::router::ModelTier::Fast => {
                                    app.router.resolver.fast_model = new_model.clone();
                                }
                                orchestrator::router::ModelTier::Balanced => {
                                    app.router.resolver.balanced_model = new_model.clone();
                                }
                                orchestrator::router::ModelTier::Capable => {
                                    app.router.resolver.capable_model = new_model.clone();
                                }
                            }
                            let short = crate::panels::sidebar::shorten_model_name_pub(&new_model);
                            app.chat.push_system(format!(
                                "Routing: {} → {}", cat.as_str(), short
                            ));
                        } else {
                            app.chat.push_system("No models available. Set an API key first.".to_string());
                        }
                    }
                }
                _ => {
                    app.sidebar_state.toggle_expand();
                }
            }
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
