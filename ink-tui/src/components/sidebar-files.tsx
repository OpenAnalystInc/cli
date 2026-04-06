/**
 * SidebarFiles — Touched files section for the sidebar panel.
 *
 * Displays files from the current session with action icons:
 *   - `○` read    (dimmed — sidebar.fileRead)
 *   - `●` edited  (yellow — sidebar.fileEdited)
 *   - `+` created (green — sidebar.fileCreated)
 *
 * Shows just the filename (not full path), truncated for 26-char width.
 * Enter toggles the file as a context file.
 */

import React from 'react';
import { Box, Text } from 'ink';
import type { FileInfo, FileAction } from '../types/messages.js';
import type { SemanticColors } from '../themes/semantic-tokens.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Max display width for filename. */
const MAX_TEXT_WIDTH = 20;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function actionIcon(action: FileAction): string {
  switch (action) {
    case 'read':    return '○';
    case 'edited':  return '●';
    case 'created': return '+';
  }
}

function actionColor(action: FileAction, colors: SemanticColors): string {
  switch (action) {
    case 'read':    return colors.sidebar.fileRead;
    case 'edited':  return colors.sidebar.fileEdited;
    case 'created': return colors.sidebar.fileCreated;
  }
}

function truncate(text: string, maxLen: number): string {
  if (text.length <= maxLen) return text;
  return text.slice(0, maxLen - 1) + '…';
}

/** Extract just the filename from a path. */
function basename(filePath: string): string {
  const sep = filePath.lastIndexOf('/');
  const bsep = filePath.lastIndexOf('\\');
  const lastSep = Math.max(sep, bsep);
  return lastSep >= 0 ? filePath.slice(lastSep + 1) : filePath;
}

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface SidebarFilesProps {
  files: readonly FileInfo[];
  selectedIndex: number;
  isFocused: boolean;
  colors: SemanticColors;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function SidebarFiles({
  files,
  selectedIndex,
  isFocused,
  colors,
}: SidebarFilesProps): React.ReactElement {
  if (files.length === 0) {
    return (
      <Text color={colors.text.secondary}>  (no files yet)</Text>
    );
  }

  return (
    <Box flexDirection="column">
      {files.map((file, i) => {
        const isSelected = isFocused && i === selectedIndex;
        const icon = actionIcon(file.action);
        const iconColor = actionColor(file.action, colors);
        const name = truncate(basename(file.path), MAX_TEXT_WIDTH);

        return (
          <Box key={file.path}>
            <Text color={iconColor}> {icon} </Text>
            <Text
              color={isSelected ? colors.sidebar.itemSelected : colors.sidebar.itemDefault}
              bold={isSelected}
            >
              {name}
            </Text>
          </Box>
        );
      })}
    </Box>
  );
}
