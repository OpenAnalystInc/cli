/**
 * PermissionDialog — modal overlay for tool permission requests.
 *
 * Centered double-border dialog matching Rust tui-widgets/permission_dialog.rs.
 * Renders over all other content with highest keypress priority (10).
 *
 * Visual structure:
 *
 *   +========================================+
 *   |     Permission Required                |
 *   | Tool: Edit                             |
 *   | Requires: danger-full-access           |
 *   |                                        |
 *   | files/app.rs:42-50 - Add error handling|
 *   |                                        |
 *   |        [ Allow ]     [ Deny ]          |
 *   +========================================+
 *
 * Keybindings (priority 10 — intercepts ALL keys except Ctrl+C):
 *   Tab / Left / Right  — switch buttons
 *   Enter               — confirm selected button
 *   Y                   — quick allow
 *   N / Esc             — quick deny
 *
 * CRITICAL: Does NOT block Ctrl+C. The global QUIT handler at priority 0
 * still fires because Ctrl+C is handled separately in the keypress
 * dispatcher before subscriber iteration.
 *
 * All colors from useTheme() semantic tokens.
 */
import React from 'react';
export declare function PermissionDialog(): React.ReactElement;
