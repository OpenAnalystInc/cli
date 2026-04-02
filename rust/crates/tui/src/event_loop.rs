//! Main event loop with `tokio::select!` over crossterm, backend, and tick events.

use std::io;
use std::time::Duration;

use ratatui::crossterm::event as ct_event;
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

    // Check for resumable sessions on startup
    app.check_resume_on_startup();

    // Crossterm event reader in a dedicated thread
    let (cx_tx, mut cx_rx) = mpsc::channel::<ct_event::Event>(64);
    std::thread::spawn(move || loop {
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
            // Crossterm keyboard/mouse events
            Some(event) = cx_rx.recv() => {
                if let ct_event::Event::Key(key) = event {
                    if key.kind == ct_event::KeyEventKind::Press {
                        handle_key(key, &mut app);
                    }
                }
                if let ct_event::Event::Mouse(mouse) = event {
                    match mouse.kind {
                        ct_event::MouseEventKind::ScrollUp => app.chat.scroll_up(3),
                        ct_event::MouseEventKind::ScrollDown => {
                            app.chat.scroll_down(3);
                        }
                        _ => {}
                    }
                }
                if matches!(event, ct_event::Event::Resize(..)) {
                    if app.chat.auto_scroll {
                        app.chat.scroll_to_bottom();
                    }
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
            break;
        }
    }

    Ok(())
}
