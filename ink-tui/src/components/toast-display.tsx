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
import { Text } from 'ink';
import { useUIState } from '../contexts/ui-state-context.js';
import { useTheme } from '../contexts/theme-context.js';

export function ToastDisplay(): React.ReactElement | null {
  const { toastMessage, toastType } = useUIState();
  const { colors } = useTheme();

  if (!toastMessage) return null;

  const colorMap = {
    info: colors.text.accent,
    warning: colors.status.warning,
    error: colors.status.error,
  } as const;

  const color = colorMap[toastType] ?? colors.text.accent;

  return (
    <Text color={color}>{toastMessage}</Text>
  );
}
