/**
 * StatusBar -- persistent single-line bar between chat and input.
 *
 * Matches Ratatui design:
 *
 *   Left side (when active):
 *     * Thinking... (4m 55s . down-arrow 5.0k tokens)
 *
 *   Right side (always):
 *     All keybinding hints in one line:
 *     Esc:input . Tab:section . j/k:nav . Ctrl+C:quit . Ctrl+B:bg . Ctrl+P:mode . F2:hide
 *
 * When idle/done the left side is hidden for a clean look.
 * The "done" checkmark auto-hides after 2 seconds.
 */
import React from 'react';
export declare function StatusBar(): React.ReactElement;
