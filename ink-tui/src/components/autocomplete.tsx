/**
 * Autocomplete — dropdown popup for `/` slash-command completion.
 *
 * Appears below the input when the user types `/`. Shows a filterable
 * list of commands with descriptions. Supports keyboard navigation.
 *
 * Keybinding priority: 7 (above input at 5, below dialogs at 9).
 *
 * Visual design:
 *   - Max 12 visible items, scrollable
 *   - Selected item: bold with accent background
 *   - Unselected items: dim
 *   - Each item shows: command name + description
 */

import React, { useMemo } from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface AutocompleteItem {
  /** Command name (e.g. "/help"). */
  name: string;
  /** Short description (e.g. "Show all commands"). */
  description: string;
}

export interface AutocompleteProps {
  /** List of available items to display. */
  items: AutocompleteItem[];
  /** Currently selected index. */
  selectedIndex: number;
  /** Whether the dropdown is visible. */
  visible: boolean;
  /** Called when an item is accepted (Tab or Enter). */
  onSelect: (item: AutocompleteItem) => void;
  /** Called when the dropdown is dismissed (Esc). */
  onDismiss: () => void;
  /** Maximum visible items before scrolling. Default: 12. */
  maxVisible?: number;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DEFAULT_MAX_VISIBLE = 12;

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function Autocomplete({
  items,
  selectedIndex,
  visible,
  maxVisible = DEFAULT_MAX_VISIBLE,
}: AutocompleteProps): React.ReactElement | null {
  const { colors } = useTheme();

  // Compute the visible window (scroll to keep selected item in view).
  const { visibleItems, startIndex } = useMemo(() => {
    if (items.length <= maxVisible) {
      return { visibleItems: items, startIndex: 0 };
    }

    // Keep the selected item roughly centered, clamped to bounds.
    let start = selectedIndex - Math.floor(maxVisible / 2);
    start = Math.max(0, start);
    start = Math.min(items.length - maxVisible, start);

    return {
      visibleItems: items.slice(start, start + maxVisible),
      startIndex: start,
    };
  }, [items, selectedIndex, maxVisible]);

  if (!visible || items.length === 0) {
    return null;
  }

  const hasScrollUp = startIndex > 0;
  const hasScrollDown = startIndex + maxVisible < items.length;

  return (
    <Box
      flexDirection="column"
      borderStyle="single"
      borderColor={colors.border.focus}
      paddingX={1}
    >
      {/* Scroll-up indicator */}
      {hasScrollUp && (
        <Text color={colors.text.secondary}>  ↑ {startIndex} more</Text>
      )}

      {/* Item list */}
      {visibleItems.map((item, i) => {
        const globalIndex = startIndex + i;
        const isSelected = globalIndex === selectedIndex;

        return (
          <Box key={item.name}>
            {isSelected ? (
              <Text backgroundColor={colors.background.focus} color={colors.text.accent} bold>
                {'▸ '}
                {item.name}
                <Text color={colors.text.secondary}> — {item.description}</Text>
              </Text>
            ) : (
              <Text color={colors.text.secondary}>
                {'  '}
                {item.name}
                <Text color={colors.text.secondary} dimColor> — {item.description}</Text>
              </Text>
            )}
          </Box>
        );
      })}

      {/* Scroll-down indicator */}
      {hasScrollDown && (
        <Text color={colors.text.secondary}>
          {'  '}↓ {items.length - startIndex - maxVisible} more
        </Text>
      )}

      {/* Navigation hints */}
      <Box marginTop={0}>
        <Text color={colors.text.secondary} dimColor>
          ↑↓ navigate · Tab accept · Esc dismiss
        </Text>
      </Box>
    </Box>
  );
}
