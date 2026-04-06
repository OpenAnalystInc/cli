/**
 * AskUserDialog — modal dialog for agent questions (choice + type modes).
 *
 * Renders over all other content with keypress priority 9.
 *
 * **Choice mode** (when options are provided):
 *
 *   +== Question ================================+
 *   | How should we handle this error?           |
 *   |                                            |
 *   |  1. Retry with backoff                     |
 *   |  2. Skip and continue        <-- selected  |
 *   |  3. Abort the operation                    |
 *   |                                            |
 *   | [T] Type . [C] Chat . Enter to select      |
 *   +============================================+
 *
 * **Type mode** (free text or toggled from choice):
 *
 *   +== Question ================================+
 *   | How should we handle this error?           |
 *   |                                            |
 *   | > user types here...                       |
 *   |                                            |
 *   | Enter to submit . Esc to go back            |
 *   +============================================+
 *
 * Keybindings (priority 9):
 *   j/k or Up/Down  — navigate options
 *   1-9             — quick-select by number
 *   Enter           — select current option or submit typed text
 *   T               — switch to type mode
 *   C               — dismiss dialog, discuss in chat
 *   Esc             — back to choice mode (from type) or dismiss
 *
 * All colors from useTheme() semantic tokens.
 */
import React from 'react';
export declare function AskUserDialog(): React.ReactElement;
