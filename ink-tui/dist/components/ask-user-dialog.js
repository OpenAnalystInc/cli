import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
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
import { useState, useCallback } from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
import { useUIState, useUIActions, } from '../contexts/ui-state-context.js';
import { useTerminal } from '../contexts/terminal-context.js';
import { useKeypress } from '../hooks/use-keypress.js';
import { Command } from '../key/commands.js';
import { useEngine } from '../engine/engine-context.js';
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
const DIALOG_WIDTH = 56;
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function AskUserDialog() {
    const { colors } = useTheme();
    const ui = useUIState();
    const actions = useUIActions();
    const terminal = useTerminal();
    const engine = useEngine();
    const dialog = ui.askUserDialog;
    const hasOptions = dialog.options !== undefined && dialog.options.length > 0;
    // Local typing state for type mode
    const [localTypedText, setLocalTypedText] = useState(dialog.typedText || '');
    const [localCursorPos, setLocalCursorPos] = useState(dialog.typedText?.length ?? 0);
    // Resolve the dialog with an answer — sends to engine and dismisses UI
    const handleResolve = useCallback((answer) => {
        engine.resolveAskUser(dialog.requestId, answer);
    }, [engine, dialog.requestId]);
    // Enter type mode
    const enterTypeMode = useCallback(() => {
        actions.showAskUserDialog({ ...dialog, typingMode: true });
    }, [actions, dialog]);
    // Return to choice mode
    const exitTypeMode = useCallback(() => {
        if (hasOptions) {
            actions.showAskUserDialog({ ...dialog, typingMode: false });
        }
    }, [actions, dialog, hasOptions]);
    // Move selection
    const moveUp = useCallback(() => {
        if (!hasOptions)
            return;
        const newIndex = Math.max(0, dialog.selectedIndex - 1);
        actions.showAskUserDialog({ ...dialog, selectedIndex: newIndex });
    }, [actions, dialog, hasOptions]);
    const moveDown = useCallback(() => {
        if (!hasOptions || !dialog.options)
            return;
        const newIndex = Math.min(dialog.options.length - 1, dialog.selectedIndex + 1);
        actions.showAskUserDialog({ ...dialog, selectedIndex: newIndex });
    }, [actions, dialog, hasOptions]);
    // Quick select by number
    const quickSelect = useCallback((num) => {
        if (!dialog.options || num < 1 || num > dialog.options.length)
            return;
        const answer = dialog.options[num - 1];
        handleResolve(answer);
    }, [dialog.options, handleResolve]);
    // Select current option
    const selectCurrent = useCallback(() => {
        if (!dialog.options || dialog.options.length === 0)
            return;
        const answer = dialog.options[dialog.selectedIndex];
        handleResolve(answer);
    }, [dialog.options, dialog.selectedIndex, handleResolve]);
    // Dismiss to chat — resolve with empty string to signal "discuss in chat"
    const dismissToChat = useCallback(() => {
        engine.resolveAskUser(dialog.requestId, '');
    }, [engine, dialog.requestId]);
    // -------------------------------------------------------------------------
    // Keypress handler — priority 9
    // -------------------------------------------------------------------------
    useKeypress(useCallback((input, key, command) => {
        // CRITICAL: Do NOT consume Ctrl+C
        if (key.ctrl && input === 'c')
            return false;
        // ── Type mode handling ──
        if (dialog.typingMode) {
            // Escape -> back to choice mode (if options exist) or dismiss
            if (key.escape) {
                if (hasOptions) {
                    exitTypeMode();
                }
                else {
                    dismissToChat();
                }
                return true;
            }
            // Enter -> submit typed text
            if (key.return) {
                const text = localTypedText.trim();
                if (text) {
                    handleResolve(text);
                }
                return true;
            }
            // Backspace
            if (key.backspace) {
                if (localCursorPos > 0) {
                    setLocalTypedText((prev) => prev.slice(0, localCursorPos - 1) + prev.slice(localCursorPos));
                    setLocalCursorPos((c) => Math.max(0, c - 1));
                }
                return true;
            }
            // Delete
            if (key.delete) {
                if (localCursorPos < localTypedText.length) {
                    setLocalTypedText((prev) => prev.slice(0, localCursorPos) + prev.slice(localCursorPos + 1));
                }
                return true;
            }
            // Arrow keys for cursor
            if (key.leftArrow) {
                setLocalCursorPos((c) => Math.max(0, c - 1));
                return true;
            }
            if (key.rightArrow) {
                setLocalCursorPos((c) => Math.min(localTypedText.length, c + 1));
                return true;
            }
            // Printable characters
            if (input && !key.ctrl && !key.meta) {
                setLocalTypedText((prev) => prev.slice(0, localCursorPos) + input + prev.slice(localCursorPos));
                setLocalCursorPos((c) => c + input.length);
                return true;
            }
            // Consume all other keys in type mode
            return true;
        }
        // ── Choice mode handling ──
        // Navigation
        if (command === Command.ASK_PREV_OPTION || input === 'k' || key.upArrow) {
            moveUp();
            return true;
        }
        if (command === Command.ASK_NEXT_OPTION || input === 'j' || key.downArrow) {
            moveDown();
            return true;
        }
        // Quick-select by number (1-9)
        if (/^[1-9]$/.test(input)) {
            quickSelect(parseInt(input, 10));
            return true;
        }
        // Enter -> select current
        if (command === Command.ASK_SELECT || key.return) {
            selectCurrent();
            return true;
        }
        // T -> switch to type mode
        if (command === Command.ASK_SWITCH_TO_TYPE || input === 't' || input === 'T') {
            if (dialog.allowFreeText) {
                enterTypeMode();
            }
            return true;
        }
        // C -> dismiss and chat about it
        if (command === Command.ASK_CHAT_ABOUT_IT || input === 'c' || input === 'C') {
            dismissToChat();
            return true;
        }
        // Escape -> dismiss
        if (key.escape) {
            dismissToChat();
            return true;
        }
        // Consume all other keys — modal blocks everything except Ctrl+C
        return true;
    }, [
        dialog.typingMode, hasOptions, localTypedText, localCursorPos,
        exitTypeMode, dismissToChat, handleResolve,
        moveUp, moveDown, quickSelect, selectCurrent, enterTypeMode,
    ]), { isActive: ui.askUserDialog !== null, priority: 9 });
    // -------------------------------------------------------------------------
    // Centering
    // -------------------------------------------------------------------------
    const dialogWidth = Math.min(DIALOG_WIDTH, terminal.width - 4);
    const padLeft = Math.max(0, Math.floor((terminal.width - dialogWidth) / 2));
    const contentLines = hasOptions ? (dialog.options?.length ?? 0) + 5 : 6;
    const dialogHeight = contentLines + 4; // borders + padding
    const padTop = Math.max(0, Math.floor((terminal.height - dialogHeight) / 2));
    // -------------------------------------------------------------------------
    // Render
    // -------------------------------------------------------------------------
    return (_jsx(Box, { position: "absolute", flexDirection: "column", marginLeft: padLeft, marginTop: padTop, children: _jsxs(Box, { flexDirection: "column", width: dialogWidth, borderStyle: "double", borderColor: colors.dialog.border, paddingX: 2, paddingY: 1, children: [_jsx(Box, { marginBottom: 1, children: _jsx(Text, { color: colors.dialog.border, bold: true, children: "Question" }) }), _jsx(Box, { marginBottom: 1, children: _jsx(Text, { color: colors.text.primary, wrap: "wrap", children: dialog.question }) }), !dialog.typingMode && hasOptions && (_jsx(Box, { flexDirection: "column", marginBottom: 1, children: dialog.options.map((option, idx) => {
                        const isSelected = idx === dialog.selectedIndex;
                        return (_jsxs(Box, { children: [_jsx(Text, { color: isSelected ? colors.text.accent : colors.text.primary, bold: isSelected, children: `  ${idx + 1}. ${option}` }), isSelected && (_jsxs(Text, { color: colors.text.accent, children: [" ", '\u2190', " selected"] }))] }, `opt-${idx}`));
                    }) })), dialog.typingMode && (_jsx(Box, { flexDirection: "column", marginBottom: 1, children: _jsxs(Box, { children: [_jsxs(Text, { color: colors.text.accent, children: ['\u276F', " "] }), _jsx(Text, { color: colors.text.primary, children: localTypedText.slice(0, localCursorPos) }), _jsx(Text, { color: "#000000", backgroundColor: colors.text.accent, children: localTypedText[localCursorPos] ?? ' ' }), _jsx(Text, { color: colors.text.primary, children: localTypedText.slice(localCursorPos + 1) })] }) })), !dialog.typingMode && !hasOptions && dialog.allowFreeText && (_jsx(Box, { flexDirection: "column", marginBottom: 1, children: _jsx(Text, { color: colors.text.secondary, dimColor: true, children: "Press any key to start typing your answer..." }) })), _jsx(Box, { children: _jsx(Text, { color: colors.text.secondary, dimColor: true, children: dialog.typingMode
                            ? `Enter to submit ${'\u00B7'} Esc to go back`
                            : hasOptions
                                ? `${dialog.allowFreeText ? '[T] Type \u00B7 ' : ''}[C] Chat ${'\u00B7'} Enter to select`
                                : `[C] Chat ${'\u00B7'} Enter to confirm` }) })] }) }));
}
//# sourceMappingURL=ask-user-dialog.js.map