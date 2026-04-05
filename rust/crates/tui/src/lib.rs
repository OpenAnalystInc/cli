//! Main TUI application for OpenAnalyst CLI.
//!
//! Full-screen TUI with alternate screen — launches fresh, exits clean back to terminal.
//! Like Claude Code: enter TUI on launch, return to shell on exit.

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
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;

/// Set up the terminal for TUI mode.
///
/// Uses alternate screen — fresh TUI on launch, clean terminal on exit.
pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);

        let sessions_dir = std::path::Path::new(".openanalyst").join("sessions");
        let _ = std::fs::create_dir_all(&sessions_dir);
        let crash_marker = sessions_dir.join("crash-recovery.marker");
        let _ = std::fs::write(&crash_marker, format!("Crash at: {:?}", panic_info.location()));

        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Restore the terminal to normal mode — back to shell.
pub fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
