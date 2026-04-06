/**
 * DiffView — renders unified diff hunks with colored +/- lines.
 *
 * Mirrors the Rust tool_card.rs diff rendering: green for added lines,
 * red for removed lines, dim for context lines, with hunk headers.
 *
 * All colors come from useTheme() semantic tokens — never hardcoded.
 */

import React from 'react';
import { Box, Text } from 'ink';
import type { DiffHunk, DiffLine } from '../types/messages.js';
import { useTheme } from '../contexts/theme-context.js';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface DiffViewProps {
  /** File path being diffed. */
  filePath: string;
  /** Number of lines added across all hunks. */
  added: number;
  /** Number of lines removed across all hunks. */
  removed: number;
  /** Diff hunks to render. */
  hunks: DiffHunk[];
  /** Maximum total lines to show (across all hunks). Defaults to 20. */
  maxLines?: number;
  /** Optional className-style width constraint. */
  maxWidth?: number;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Truncate a string to maxWidth, appending ellipsis if needed. */
function truncate(s: string, maxWidth: number): string {
  if (s.length <= maxWidth) return s;
  if (maxWidth > 3) return s.slice(0, maxWidth - 3) + '...';
  return s.slice(0, maxWidth);
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function DiffView({
  filePath,
  added,
  removed,
  hunks,
  maxLines = 20,
  maxWidth,
}: DiffViewProps): React.ReactElement {
  const { colors } = useTheme();

  // Flatten all lines from all hunks, inserting hunk headers.
  const flatLines: { type: 'header' | 'added' | 'removed' | 'context'; text: string }[] = [];

  for (const hunk of hunks) {
    flatLines.push({
      type: 'header',
      text: `@@ -${hunk.oldStart} +${hunk.newStart} @@`,
    });
    for (const line of hunk.lines) {
      flatLines.push({
        type: line.kind,
        text: line.kind === 'added'
          ? `+ ${line.text}`
          : line.kind === 'removed'
            ? `- ${line.text}`
            : `  ${line.text}`,
      });
    }
  }

  const totalLines = flatLines.length;
  const visibleLines = flatLines.slice(0, maxLines);
  const overflowCount = totalLines - maxLines;

  return (
    <Box flexDirection="column">
      {/* File path + stats header */}
      <Text>
        <Text color={colors.text.accent} bold>{filePath}</Text>
        <Text color={colors.text.secondary}> </Text>
        <Text color={colors.diff.added} bold>+{added}</Text>
        <Text color={colors.text.secondary}> / </Text>
        <Text color={colors.diff.removed} bold>-{removed}</Text>
      </Text>

      {/* Diff lines */}
      {visibleLines.map((line, i) => {
        const displayText = maxWidth ? truncate(line.text, maxWidth) : line.text;

        if (line.type === 'header') {
          return (
            <Text key={i} color={colors.text.accent} dimColor>
              {displayText}
            </Text>
          );
        }

        if (line.type === 'added') {
          return (
            <Text key={i} color={colors.diff.added}>
              {displayText}
            </Text>
          );
        }

        if (line.type === 'removed') {
          return (
            <Text key={i} color={colors.diff.removed}>
              {displayText}
            </Text>
          );
        }

        // Context line
        return (
          <Text key={i} color={colors.text.secondary}>
            {displayText}
          </Text>
        );
      })}

      {/* Overflow indicator */}
      {overflowCount > 0 && (
        <Text color={colors.text.secondary} dimColor>
          ... ({overflowCount} more lines)
        </Text>
      )}
    </Box>
  );
}
