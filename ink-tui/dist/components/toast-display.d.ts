/**
 * ToastDisplay -- brief notification line that auto-hides.
 *
 * Rendered above the status bar. Shows feedback messages like:
 *   - "Copied to clipboard"
 *   - "Permission mode: Accept Edits"
 *   - "Context file added"
 *   - "Session saved"
 *
 * Auto-dismisses via the UIState toast timer.
 * All colors from useTheme() semantic tokens.
 */
import React from 'react';
export declare function ToastDisplay(): React.ReactElement | null;
