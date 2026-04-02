//! Panel layout calculations for the full-screen scrollable TUI.

use ratatui::layout::{Constraint, Layout, Rect};

/// Computed layout regions for the TUI.
pub struct AppLayout {
    /// Main scrollable chat area.
    pub chat: Rect,
    /// Status line between chat and input.
    pub status: Rect,
    /// Input prompt at the bottom.
    pub input: Rect,
}

/// Compute the layout for the TUI.
///
/// Full-screen scrollable layout (OpenAnalyst TUI):
/// ```text
/// ┌──────────────────────────┐
/// │  Chat area (scrollable)  │  Min height
/// ├──────────────────────────┤
/// │  Status line             │  1 row
/// ├──────────────────────────┤
/// │  Input prompt            │  3 rows
/// └──────────────────────────┘
/// ```
pub fn compute_layout(area: Rect) -> AppLayout {
    let chunks = Layout::vertical([
        Constraint::Min(5),     // chat area (takes remaining space)
        Constraint::Length(1),  // status line
        Constraint::Length(3),  // input area
    ])
    .split(area);

    AppLayout {
        chat: chunks[0],
        status: chunks[1],
        input: chunks[2],
    }
}
