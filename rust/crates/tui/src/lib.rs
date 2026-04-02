//! Main TUI application for OpenAnalyst CLI.
//!
//! Full-screen scrollable chat interface with scrollable layout,
//! startup banner, inline tool cards, status line, and multi-agent support.

pub mod app;
pub mod autocomplete;
pub mod banner;
pub mod event_loop;
pub mod keybindings;
pub mod layout;
pub mod panels;
pub mod slash_commands;
pub mod theme;

use std::io;

use ratatui::crossterm::execute;
use ratatui::crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;

/// Set up the terminal for TUI mode.
pub fn setup_terminal() -> io::Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    Terminal::new(backend)
}

/// Restore the terminal to normal mode.
pub fn restore_terminal() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
