/**
 * DefaultLayout — main layout component for the OpenAnalyst TUI.
 *
 * Panel arrangement (matching Rust crate tui::layout):
 *
 *   +-------------------------------+--------+
 *   |                               |        |
 *   |         Chat Panel            | Sidebar|
 *   |         (flex grow)           | (26ch) |
 *   |                               |        |
 *   +-------------------------------+--------+
 *   | Status bar (1 line)                     |
 *   +----------------------------------------+
 *   | Input box (5-10 lines dynamic)          |
 *   +----------------------------------------+
 *
 * Layout constraints from Rust:
 *   - SIDEBAR_WIDTH = 26
 *   - INPUT_MIN_HEIGHT = 5, INPUT_MAX_HEIGHT = 10
 *   - Sidebar hidden when terminal width < 60
 *   - Chat panel minimum width = 30
 */
import React from 'react';
/** Minimum input box height (including border). */
/** Maximum input box height (including border). */
export declare function DefaultLayout(): React.ReactElement;
