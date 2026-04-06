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

import React, { useCallback } from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
import {
  useUIState,
  useUIActions,
  type PermissionDialogState,
} from '../contexts/ui-state-context.js';
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
function permissionModeLabel(mode: string): string {
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

export function PermissionDialog(): React.ReactElement {
  const { colors } = useTheme();
  const ui = useUIState();
  const actions = useUIActions();
  const terminal = useTerminal();
  const engine = useEngine();

  const dialog = ui.permissionDialog!;

  // Resolve action — sends decision to engine and dismisses UI dialog
  const handleResolve = useCallback(
    (decision: 'allow' | 'deny') => {
      engine.resolvePermission(dialog.requestId, decision);
    },
    [engine, dialog.requestId],
  );

  // Toggle button selection
  const toggleButton = useCallback(() => {
    if (!ui.permissionDialog) return;
    const newSelected = dialog.selectedButton === 'allow' ? 'deny' : 'allow';
    actions.showPermissionDialog({ ...dialog, selectedButton: newSelected });
  }, [actions, dialog, ui.permissionDialog]);

  // Confirm selected button
  const confirmSelection = useCallback(() => {
    handleResolve(dialog.selectedButton);
  }, [dialog.selectedButton, handleResolve]);

  // Keypress handler — priority 10 (highest modal priority)
  useKeypress(
    useCallback((input, key, command) => {
      // CRITICAL: Do NOT consume Ctrl+C — let it propagate to the global
      // QUIT handler. Ink's useInput provides ctrl=true for Ctrl+C.
      if (key.ctrl && input === 'c') return false;

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
    }, [toggleButton, confirmSelection, handleResolve]),
    { isActive: ui.permissionDialog !== null, priority: 10 },
  );

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

  return (
    <Box
      position="absolute"
      flexDirection="column"
      marginLeft={padLeft}
      marginTop={padTop}
    >
      <Box
        flexDirection="column"
        width={dialogWidth}
        borderStyle="double"
        borderColor={colors.dialog.border}
        paddingX={2}
        paddingY={1}
      >
        {/* Title */}
        <Box justifyContent="center" marginBottom={1}>
          <Text color={colors.dialog.border} bold>
            Permission Required
          </Text>
        </Box>

        {/* Tool name */}
        <Box>
          <Text color={colors.text.secondary}>Tool: </Text>
          <Text color={colors.text.accent} bold>{dialog.toolName}</Text>
        </Box>

        {/* Required permission mode */}
        <Box>
          <Text color={colors.text.secondary}>Requires: </Text>
          <Text color={colors.dialog.border}>
            {permissionModeLabel(dialog.requiredMode)}
          </Text>
        </Box>

        {/* Description / file path preview */}
        {(dialog.filePath || dialog.description) && (
          <Box marginTop={1} flexDirection="column">
            {dialog.filePath && (
              <Text color={colors.text.primary}>{dialog.filePath}</Text>
            )}
            {dialog.description && (
              <Text color={colors.text.secondary} wrap="wrap">
                {dialog.description}
              </Text>
            )}
          </Box>
        )}

        {/* Tool input preview (truncated) */}
        {dialog.toolInput && (
          <Box marginTop={1}>
            <Text color={colors.text.primary} wrap="truncate-end">
              {dialog.toolInput.slice(0, dialogWidth - 8)}
            </Text>
          </Box>
        )}

        {/* Buttons */}
        <Box justifyContent="center" marginTop={1} gap={4}>
          <Text backgroundColor={allowBg} color={allowTextColor} bold={dialog.selectedButton === 'allow'}>
            {'  [ Allow ]  '}
          </Text>
          <Text backgroundColor={denyBg} color={denyTextColor} bold={dialog.selectedButton === 'deny'}>
            {'  [ Deny ]  '}
          </Text>
        </Box>

        {/* Hint */}
        <Box justifyContent="center" marginTop={1}>
          <Text color={colors.text.secondary} dimColor>
            Y=allow {'\u00B7'} N=deny {'\u00B7'} Tab=switch {'\u00B7'} Enter=confirm
          </Text>
        </Box>
      </Box>
    </Box>
  );
}
