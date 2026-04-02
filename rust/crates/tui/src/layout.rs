//! Panel layout calculations for the full-screen TUI.
//!
//! Supports a collapsible right sidebar and dynamic input box height.

use ratatui::layout::{Constraint, Layout, Rect};

/// Width of the sidebar when visible.
pub const SIDEBAR_WIDTH: u16 = 26;

/// Minimum input box height (borders take 2 lines, so 5 = 3 lines for typing).
pub const INPUT_MIN_HEIGHT: u16 = 5;

/// Maximum input box height.
pub const INPUT_MAX_HEIGHT: u16 = 10;

/// Computed layout regions for the TUI.
pub struct AppLayout {
    /// Main scrollable chat area.
    pub chat: Rect,
    /// Status line between chat and input (full width).
    pub status: Rect,
    /// Input prompt at the bottom (full width).
    pub input: Rect,
    /// Optional sidebar area (None if collapsed).
    pub sidebar: Option<Rect>,
}

/// Compute the layout for the TUI.
///
/// `input_height` is dynamically calculated from current editor line count.
pub fn compute_layout(area: Rect, sidebar_visible: bool, input_height: u16) -> AppLayout {
    let clamped_input = input_height.clamp(INPUT_MIN_HEIGHT, INPUT_MAX_HEIGHT);

    // Vertical split: chat | status | input
    let rows = Layout::vertical([
        Constraint::Min(5),              // chat + sidebar row
        Constraint::Length(1),           // status line
        Constraint::Length(clamped_input), // input area (dynamic)
    ])
    .split(area);

    let (chat, sidebar) = if sidebar_visible && area.width > 60 {
        let cols = Layout::horizontal([
            Constraint::Min(30),
            Constraint::Length(SIDEBAR_WIDTH),
        ])
        .split(rows[0]);
        (cols[0], Some(cols[1]))
    } else {
        (rows[0], None)
    };

    AppLayout {
        chat,
        status: rows[1],
        input: rows[2],
        sidebar,
    }
}
