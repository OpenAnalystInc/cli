/**
 * ContextFileTags — renders @filename badges in the input box bottom border.
 *
 * Files are shown as cyan-on-dark badges. When the total width exceeds
 * the available space, remaining files are collapsed into a "+N more" indicator.
 *
 * All colors from useTheme() semantic tokens.
 */

import React from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface ContextFileTagsProps {
  /** Full file paths — only the filename portion is displayed. */
  files: string[];
  /** Maximum available width in columns for the tag row. */
  maxWidth: number;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Extract the filename from a full path. */
function basename(filePath: string): string {
  const parts = filePath.split(/[/\\]/);
  return parts[parts.length - 1] ?? filePath;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function ContextFileTags({
  files,
  maxWidth,
}: ContextFileTagsProps): React.ReactElement | null {
  const { colors } = useTheme();

  if (files.length === 0) return null;

  // Calculate which files fit within maxWidth
  const badges: Array<{ label: string; key: string }> = [];
  let usedWidth = 0;
  let shownCount = 0;

  for (const filePath of files) {
    const name = basename(filePath);
    // Badge format: " @filename " + 1 space separator
    const badgeWidth = name.length + 3 + 1; // " @" + name + " " + separator

    // Reserve space for "+N more" indicator
    const remaining = files.length - shownCount;
    const moreIndicatorWidth = remaining > 1 ? ` +${remaining - 1} more `.length + 1 : 0;

    if (usedWidth + badgeWidth + moreIndicatorWidth > maxWidth && shownCount > 0) {
      break;
    }

    badges.push({ label: `@${name}`, key: filePath });
    usedWidth += badgeWidth;
    shownCount++;
  }

  const hiddenCount = files.length - shownCount;

  return (
    <Box gap={1}>
      {badges.map(({ label, key }) => (
        <Text
          key={key}
          color={colors.text.accent}
          backgroundColor={colors.background.badge.contextFile}
        >
          {` ${label} `}
        </Text>
      ))}
      {hiddenCount > 0 && (
        <Text color={colors.text.secondary} dimColor>
          {` +${hiddenCount} more `}
        </Text>
      )}
    </Box>
  );
}
