/**
 * FeedbackWidget — inline feedback prompt rendered below a KnowledgeCard.
 *
 * Mirrors the Rust tui-widgets/feedback_dialog.rs:
 *   Was this helpful?  [Y] [N] [Esc dismiss]
 *
 * Selected button has bold text + background color.
 * Unselected buttons have plain colored text.
 *
 * Keybinding: y/n/Esc in scroll mode (connected via useKeypress).
 * All colors from useTheme() semantic tokens.
 */

import React, { useCallback } from 'react';
import { Box, Text } from 'ink';
import type { Key as InkKey } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
import { useKeypress } from '../hooks/use-keypress.js';
import type { Command } from '../key/commands.js';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface FeedbackWidgetProps {
  /** Query ID to attach feedback to. */
  queryId: string;
  /** Currently selected button: 0=positive, 1=negative, 2=dismiss. */
  selectedIndex: number;
  /** Callback when a selection is made. */
  onSelect: (rating: 'positive' | 'negative' | 'dismiss') => void;
  /** Callback when selection changes (for cycling). */
  onSelectionChange: (index: number) => void;
  /** Whether this widget is active (receives keypresses). */
  isActive: boolean;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function FeedbackWidget({
  queryId,
  selectedIndex,
  onSelect,
  onSelectionChange,
  isActive,
}: FeedbackWidgetProps): React.ReactElement {
  const { colors } = useTheme();

  // Handle key presses when active.
  useKeypress(
    useCallback(
      (ch: string, key: InkKey, _command: Command | undefined) => {
        if (!isActive) return false;

        // Direct shortcuts.
        if (ch === 'y' || ch === 'Y') {
          onSelect('positive');
          return true;
        }
        if (ch === 'n' || ch === 'N') {
          onSelect('negative');
          return true;
        }
        if (key.escape) {
          onSelect('dismiss');
          return true;
        }

        // Tab/arrow cycling.
        if (key.tab || key.rightArrow) {
          onSelectionChange((selectedIndex + 1) % 3);
          return true;
        }
        if (key.leftArrow) {
          onSelectionChange(selectedIndex === 0 ? 2 : selectedIndex - 1);
          return true;
        }

        // Enter confirms current selection.
        if (key.return) {
          const ratings = ['positive', 'negative', 'dismiss'] as const;
          onSelect(ratings[selectedIndex] ?? 'dismiss');
          return true;
        }

        return false;
      },
      [isActive, selectedIndex, onSelect, onSelectionChange],
    ),
    { isActive, priority: 60 },
  );

  // Button styles — selected gets background + bold, unselected is plain text.
  const positiveStyle = selectedIndex === 0
    ? { color: '#000000' as string, backgroundColor: colors.status.done, bold: true }
    : { color: colors.status.done, bold: false };

  const negativeStyle = selectedIndex === 1
    ? { color: '#000000' as string, backgroundColor: colors.status.error, bold: true }
    : { color: colors.status.error, bold: false };

  const dismissStyle = selectedIndex === 2
    ? { color: '#000000' as string, backgroundColor: colors.text.secondary, bold: true }
    : { color: colors.text.secondary, bold: false };

  return (
    <Box paddingLeft={2}>
      <Text color={colors.text.primary}>Was this helpful? </Text>

      {/* Positive button */}
      <Text
        color={positiveStyle.color}
        backgroundColor={positiveStyle.backgroundColor}
        bold={positiveStyle.bold}
      >
        {' Y '}
      </Text>
      <Text> </Text>

      {/* Negative button */}
      <Text
        color={negativeStyle.color}
        backgroundColor={negativeStyle.backgroundColor}
        bold={negativeStyle.bold}
      >
        {' N '}
      </Text>
      <Text> </Text>

      {/* Dismiss button */}
      <Text
        color={dismissStyle.color}
        backgroundColor={dismissStyle.backgroundColor}
        bold={dismissStyle.bold}
      >
        {' Esc '}
      </Text>

      <Text color={colors.text.secondary} dimColor>
        {' '}{'·'} /feedback for corrections
      </Text>
    </Box>
  );
}
