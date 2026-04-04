//! Main TUI application for OpenAnalyst CLI.
//!
//! Inline viewport chat interface with scrollable layout,
//! startup banner, inline tool cards, status line, and multi-agent support.
//! Uses the terminal's normal buffer (not alternate screen) so the native
//! scrollbar works for scrolling through conversation history.

pub mod app;
pub mod autocomplete;
pub mod banner;
pub mod event_loop;
pub mod keybinding_config;
pub mod keybindings;
pub mod layout;
pub mod panels;
pub mod slash_commands;
pub mod theme;
pub mod voice;

use std::io;

use ratatui::crossterm::event::{EnableMouseCapture, DisableMouseCapture};
use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode,
};
use ratatui::prelude::CrosstermBackend;
use ratatui::{Terminal, TerminalOptions, Viewport};

/// Set up the terminal for TUI mode with a panic handler that restores the terminal.
///
/// Uses inline viewport (not alternate screen) so the terminal's native scrollbar
/// works for scrolling through conversation history — matching Claude Code behavior.
pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    // Install panic hook that restores the terminal before printing the panic message.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        // Best-effort terminal restoration — ignore errors since we're already panicking
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), DisableMouseCapture);

        // Auto-save session on crash (best-effort)
        let sessions_dir = std::path::Path::new(".openanalyst").join("sessions");
        let _ = std::fs::create_dir_all(&sessions_dir);
        let crash_marker = sessions_dir.join("crash-recovery.marker");
        let _ = std::fs::write(&crash_marker, format!("Crash at: {:?}", panic_info.location()));

        // Print the original panic message to the restored terminal
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnableMouseCapture)?;

    // Use inline viewport at terminal height — renders in normal buffer
    // so the terminal's native scrollbar works for scrolling history
    let (_, rows) = ratatui::crossterm::terminal::size()?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::with_options(
        backend,
        TerminalOptions {
            viewport: Viewport::Inline(rows),
        },
    )
}

/// Restore the terminal to normal mode.
pub fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), DisableMouseCapture)?;
    Ok(())
}
