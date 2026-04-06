import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
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
import { useCallback } from 'react';
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
const DIALOG_MIN_HEIGHT = 12;
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
/** Human-readable label for permission modes. */
function permissionModeLabel(mode) {
    switch (mode) {
        case 'prompt': return 'Default (prompt)';
        case 'read-only': return 'Read Only';
        case 'workspace-write': return 'Workspace Write';
        case 'danger-full-access': return 'Danger (full access)';
        default: return mode;
    }
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function PermissionDialog() {
    const { colors } = useTheme();
    const ui = useUIState();
    const actions = useUIActions();
    const terminal = useTerminal();
    const engine = useEngine();
    const dialog = ui.permissionDialog;
    // Resolve action — sends decision to engine and dismisses UI dialog
    const handleResolve = useCallback((decision) => {
        engine.resolvePermission(dialog.requestId, decision);
    }, [engine, dialog.requestId]);
    // Toggle button selection
    const toggleButton = useCallback(() => {
        if (!ui.permissionDialog)
            return;
        const newSelected = dialog.selectedButton === 'allow' ? 'deny' : 'allow';
        actions.showPermissionDialog({ ...dialog, selectedButton: newSelected });
    }, [actions, dialog, ui.permissionDialog]);
    // Confirm selected button
    const confirmSelection = useCallback(() => {
        handleResolve(dialog.selectedButton);
    }, [dialog.selectedButton, handleResolve]);
    // Keypress handler — priority 10 (highest modal priority)
    useKeypress(useCallback((input, key, command) => {
        // CRITICAL: Do NOT consume Ctrl+C — let it propagate to the global
        // QUIT handler. Ink's useInput provides ctrl=true for Ctrl+C.
        if (key.ctrl && input === 'c')
            return false;
        // Tab or arrow keys switch buttons
        if (command === Command.DIALOG_SWITCH_BUTTON || key.tab || key.leftArrow || key.rightArrow) {
            toggleButton();
            return true;
        }
        // Enter confirms the selected button
        if (command === Command.DIALOG_CONFIRM || key.return) {
            confirmSelection();
            return true;
        }
        // Y = quick allow
        if (command === Command.DIALOG_ALLOW || input === 'y' || input === 'Y') {
            handleResolve('allow');
            return true;
        }
        // N or Esc = quick deny
        if (command === Command.DIALOG_DENY || input === 'n' || input === 'N' || key.escape) {
            handleResolve('deny');
            return true;
        }
        // Consume all other keys — modal blocks everything except Ctrl+C
        return true;
    }, [toggleButton, confirmSelection, handleResolve]), { isActive: ui.permissionDialog !== null, priority: 10 });
    // -------------------------------------------------------------------------
    // Centering
    // -------------------------------------------------------------------------
    const dialogWidth = Math.min(DIALOG_WIDTH, terminal.width - 4);
    const padLeft = Math.max(0, Math.floor((terminal.width - dialogWidth) / 2));
    const padTop = Math.max(0, Math.floor((terminal.height - DIALOG_MIN_HEIGHT) / 2));
    // -------------------------------------------------------------------------
    // Button styling
    // -------------------------------------------------------------------------
    const allowBg = dialog.selectedButton === 'allow'
        ? colors.dialog.allowSelected
        : colors.dialog.allowUnselected;
    const denyBg = dialog.selectedButton === 'deny'
        ? colors.dialog.denySelected
        : colors.dialog.denyUnselected;
    const allowTextColor = dialog.selectedButton === 'allow' ? '#000000' : colors.status.done;
    const denyTextColor = dialog.selectedButton === 'deny' ? '#000000' : colors.status.error;
    // -------------------------------------------------------------------------
    // Render
    // -------------------------------------------------------------------------
    return (_jsx(Box, { position: "absolute", flexDirection: "column", marginLeft: padLeft, marginTop: padTop, children: _jsxs(Box, { flexDirection: "column", width: dialogWidth, borderStyle: "double", borderColor: colors.dialog.border, paddingX: 2, paddingY: 1, children: [_jsx(Box, { justifyContent: "center", marginBottom: 1, children: _jsx(Text, { color: colors.dialog.border, bold: true, children: "Permission Required" }) }), _jsxs(Box, { children: [_jsx(Text, { color: colors.text.secondary, children: "Tool: " }), _jsx(Text, { color: colors.text.accent, bold: true, children: dialog.toolName })] }), _jsxs(Box, { children: [_jsx(Text, { color: colors.text.secondary, children: "Requires: " }), _jsx(Text, { color: colors.dialog.border, children: permissionModeLabel(dialog.requiredMode) })] }), (dialog.filePath || dialog.description) && (_jsxs(Box, { marginTop: 1, flexDirection: "column", children: [dialog.filePath && (_jsx(Text, { color: colors.text.primary, children: dialog.filePath })), dialog.description && (_jsx(Text, { color: colors.text.secondary, wrap: "wrap", children: dialog.description }))] })), dialog.toolInput && (_jsx(Box, { marginTop: 1, children: _jsx(Text, { color: colors.text.primary, wrap: "truncate-end", children: dialog.toolInput.slice(0, dialogWidth - 8) }) })), _jsxs(Box, { justifyContent: "center", marginTop: 1, gap: 4, children: [_jsx(Text, { backgroundColor: allowBg, color: allowTextColor, bold: dialog.selectedButton === 'allow', children: '  [ Allow ]  ' }), _jsx(Text, { backgroundColor: denyBg, color: denyTextColor, bold: dialog.selectedButton === 'deny', children: '  [ Deny ]  ' })] }), _jsx(Box, { justifyContent: "center", marginTop: 1, children: _jsxs(Text, { color: colors.text.secondary, dimColor: true, children: ["Y=allow ", '\u00B7', " N=deny ", '\u00B7', " Tab=switch ", '\u00B7', " Enter=confirm"] }) })] }) }));
}
//# sourceMappingURL=permission-dialog.js.map