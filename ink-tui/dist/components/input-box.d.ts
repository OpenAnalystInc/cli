/**
 * InputBox -- minimal input area matching the Ratatui design.
 *
 * Visual structure:
 *
 *   [icon] Enter to send . Ctrl+P mode  [I] --------[No-Git]
 *   |  user types here...
 *   |                                           API credits
 *
 * Features:
 *   - Top line: prompt icon + hint text + vim indicator + horizontal rule + branch badge
 *   - The horizontal rule color changes by permission mode (blue/yellow/green/red)
 *   - Below: clean text input area with NO box border
 *   - Bottom-right: credit balance + MCP count
 *   - Multi-line text input with basic editing
 *   - Vim mode: normal / insert mode tracking
 *   - History navigation (Up/Down)
 *   - Enter to submit, dynamic height 3-8 lines
 *   - Disabled state during streaming/agent running
 *
 * All colors from useTheme() semantic tokens.
 * Keypress subscription at priority 3 (input mode).
 */
import React from 'react';
export interface InputBoxProps {
    /** Callback when the user submits a prompt. */
    onSubmit?: (text: string) => void;
}
export declare function InputBox({ onSubmit }: InputBoxProps): React.ReactElement;
