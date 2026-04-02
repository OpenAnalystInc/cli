//! Main event loop with `tokio::select!` over crossterm, backend, and tick events.

use std::io;
use std::time::Duration;

use ratatui::crossterm::event as ct_event;
use ratatui::crossterm::execute;
use ratatui::crossterm::event::{EnableBracketedPaste, DisableBracketedPaste};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::app::App;
use crate::keybindings::handle_key;

/// Run the TUI event loop.
pub async fn run_event_loop(
    mut app: App,
    mut terminal: Terminal<CrosstermBackend<io::Stdout>>,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(100);
    let mut tick_interval = tokio::time::interval(tick_rate);

    // Auto-save session every 60 seconds
    let mut auto_save_interval = tokio::time::interval(Duration::from_secs(60));

    // Enable bracketed paste so we can receive multi-line paste as a single event
    let _ = execute!(io::stdout(), EnableBracketedPaste);

    // Check for resumable sessions on startup
    app.check_resume_on_startup();

    // Crossterm event reader in a dedicated thread
    let (cx_tx, mut cx_rx) = mpsc::channel::<ct_event::Event>(64);
    let crossterm_thread = std::thread::spawn(move || loop {
        if ct_event::poll(Duration::from_millis(50)).unwrap_or(false) {
            if let Ok(event) = ct_event::read() {
                if cx_tx.blocking_send(event).is_err() {
                    break;
                }
            }
        }
    });

    loop {
        // Draw
        terminal.draw(|frame| {
            app.render(frame.area(), frame.buffer_mut());
        })?;

        // Handle events
        tokio::select! {
            // Crossterm keyboard/mouse/paste events
            Some(event) = cx_rx.recv() => {
                match event {
                    ct_event::Event::Key(key) if key.kind == ct_event::KeyEventKind::Press => {
                        handle_key(key, &mut app);
                    }
                    ct_event::Event::Mouse(mouse) => {
                        match mouse.kind {
                            ct_event::MouseEventKind::ScrollUp => app.chat.scroll_up(3),
                            ct_event::MouseEventKind::ScrollDown => app.chat.scroll_down(3),
                            _ => {}
                        }
                    }
                    // Handle pasted text — file paths, multi-line content, anything
                    ct_event::Event::Paste(text) => {
                        handle_paste(&mut app, &text);
                    }
                    ct_event::Event::Resize(..) => {
                        if app.chat.auto_scroll {
                            app.chat.scroll_to_bottom();
                        }
                    }
                    _ => {}
                }
            }
            // Backend events (streaming, tool calls, agent status)
            Some(event) = app.ui_rx.recv() => {
                app.handle_ui_event(event);
            }
            // Animation tick
            _ = tick_interval.tick() => {
                app.tick();
            }
            // Auto-save session periodically
            _ = auto_save_interval.tick() => {
                app.auto_save_session();
            }
        }

        if app.should_quit {
            // Final save before exit
            app.auto_save_session();
            let _ = app.action_tx.try_send(events::Action::Quit);
            // Disable bracketed paste before exiting
            let _ = execute!(io::stdout(), DisableBracketedPaste);
            break;
        }
    }

    // Drop the receiver so the crossterm thread's blocking_send returns Err and it exits.
    drop(cx_rx);
    // Join the crossterm event thread for clean shutdown.
    let _ = crossterm_thread.join();

    Ok(())
}

/// Handle pasted text — supports file paths, multi-line content, image paths.
///
/// If the pasted text looks like a file path (image, audio, code file),
/// it's wrapped in a reference format the model can understand.
fn handle_paste(app: &mut App, text: &str) {
    let trimmed = text.trim();

    if trimmed.is_empty() {
        return;
    }

    // Detect if it's a file path (single line, looks like a path)
    if !trimmed.contains('\n') && looks_like_file_path(trimmed) {
        let path = std::path::Path::new(trimmed);
        if path.exists() {
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            let tag = match ext.to_ascii_lowercase().as_str() {
                "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" => "[image]",
                "mp3" | "wav" | "ogg" | "m4a" | "flac" => "[audio]",
                "mp4" | "mov" | "avi" | "mkv" | "webm" => "[video]",
                "pdf" => "[document]",
                _ => "[file]",
            };
            // Insert as a tagged reference
            let reference = format!("{tag} {trimmed}");
            app.input_state.set_text(&reference);
            app.suggestions.dismiss();
            return;
        }
    }

    // Regular text paste — insert into input box
    // For multi-line paste, set the full text
    app.input_state.set_text(trimmed);
    // Update autocomplete if it starts with /
    app.suggestions.update(trimmed);
}

/// Heuristic: does this string look like a file path?
fn looks_like_file_path(s: &str) -> bool {
    // Contains path separator
    if s.contains('/') || s.contains('\\') {
        return true;
    }
    // Has a file extension
    if let Some(dot_pos) = s.rfind('.') {
        let ext = &s[dot_pos + 1..];
        if ext.len() >= 1 && ext.len() <= 5 && ext.chars().all(|c| c.is_ascii_alphanumeric()) {
            return true;
        }
    }
    // Starts with a drive letter (Windows)
    if s.len() >= 3 && s.as_bytes()[1] == b':' && (s.as_bytes()[2] == b'\\' || s.as_bytes()[2] == b'/') {
        return true;
    }
    false
}
