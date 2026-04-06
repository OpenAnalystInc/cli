/**
 * SidebarRouting -- Model routing table section for the sidebar panel.
 *
 * Matches Ratatui design:
 *   explore   . model-name
 *   research  . model-name
 *   code      . model-name
 *   write     . model-name
 *
 * Category colors: explore=cyan, research=yellow, code=green, write=orange
 * Tier dot color: fast=cyan, balanced=yellow, capable=green
 * Enter cycles the tier for the selected category.
 */

import React from 'react';
import { Box, Text } from 'ink';
import type { RoutingTable, ActionCategory } from '../types/messages.js';
import type { SemanticColors } from '../themes/semantic-tokens.js';
import { providerPreferences } from '../utils/provider-preferences.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** The 4 routing categories in display order. */
const CATEGORIES: readonly ActionCategory[] = ['explore', 'research', 'code', 'write'];

/** Display labels (lowercase, padded to 10 chars for alignment). */
const CATEGORY_LABELS: Record<ActionCategory, string> = {
  explore:  'explore   ',
  research: 'research  ',
  code:     'code      ',
  write:    'write     ',
};

/**
 * Category-specific colors matching Ratatui sidebar.
 * Resolved at render time from semantic tokens via getCategoryColor().
 */

/** Map categories to semantic token colors. */
function getCategoryColor(cat: ActionCategory, colors: SemanticColors): string {
  switch (cat) {
    case 'explore':  return colors.text.accent;         // cyan
    case 'research': return colors.status.warning;      // yellow
    case 'code':     return colors.status.done;         // green
    case 'write':    return colors.text.slashCommand;   // orange
  }
}

/** Map tier to semantic token colors. */
function getTierDotColor(tier: string, colors: SemanticColors): string {
  switch (tier) {
    case 'fast':     return colors.text.accent;         // cyan
    case 'balanced': return colors.status.warning;      // yellow
    case 'capable':  return colors.status.done;         // green
    default:         return colors.text.secondary;      // dim
  }
}

/** Max model name width for 26-char sidebar. */
const MAX_MODEL_WIDTH = 10;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function truncate(text: string, maxLen: number): string {
  if (text.length <= maxLen) return text;
  return text.slice(0, maxLen - 1) + '\u2026';
}


// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface SidebarRoutingProps {
  routing: RoutingTable;
  selectedIndex: number;
  isFocused: boolean;
  colors: SemanticColors;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function SidebarRouting({
  routing,
  selectedIndex,
  isFocused,
  colors,
}: SidebarRoutingProps): React.ReactElement {
  // Resolve default provider for display
  const defaultProvider = providerPreferences.getDefaultProvider();
  const defaultConfig = defaultProvider ? providerPreferences.getDefaultModelForProvider(defaultProvider) : null;

  return (
    <Box flexDirection="column">
      {/* Show default provider hint */}
      {defaultProvider && defaultConfig && (
        <Box>
          <Text color={colors.status.warning}>{' \u2605 '}</Text>
          <Text color={colors.text.secondary} dimColor>
            {truncate(defaultConfig.name, MAX_MODEL_WIDTH)}
          </Text>
        </Box>
      )}
      {CATEGORIES.map((cat, i) => {
        const entry = routing[cat];
        const isSelected = isFocused && i === selectedIndex;
        const label = CATEGORY_LABELS[cat];

        // If model name is empty, try to resolve from saved preferences or default provider
        let modelDisplay = entry.model || entry.tier;
        if (!modelDisplay && defaultConfig) {
          modelDisplay = defaultConfig.name;
        }
        const model = truncate(modelDisplay || 'beta', MAX_MODEL_WIDTH);

        const catColor = getCategoryColor(cat, colors);
        const dotColor = getTierDotColor(entry.tier || defaultConfig?.tier || 'balanced', colors);

        const selPrefix = isSelected ? '\u25B8' : ' ';
        const bg = isSelected ? colors.sidebar.border : undefined;

        return (
          <Box key={cat}>
            <Text color={colors.status.warning} backgroundColor={bg}>{selPrefix}</Text>
            <Text color={catColor} backgroundColor={bg}>
              {label}
            </Text>
            <Text color={dotColor} backgroundColor={bg}>
              {'\u25CF'} </Text>
            <Text color={colors.text.secondary} backgroundColor={bg}>
              {model}
            </Text>
          </Box>
        );
      })}
    </Box>
  );
}
