/**
 * FileOutput — renders file output messages (image, audio, diagram, text).
 *
 * Visual structure:
 *   [IMG] Description of the generated image
 *         /path/to/output/file.png
 *
 * Type badges:
 *   IMG  — green (images)
 *   AUD  — blue (audio files)
 *   DGM  — cyan (diagrams)
 *   TXT  — dimmed (text files)
 *
 * All colors from useTheme() semantic tokens.
 */

import React from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
import type { FileOutputType } from '../types/chat.js';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface FileOutputProps {
  fileType: FileOutputType;
  description: string;
  filePath: string;
  isFocused: boolean;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

interface TypeBadge {
  label: string;
  colorKey: 'done' | 'pending' | 'warning' | 'error';
}

const TYPE_BADGES: Record<FileOutputType, TypeBadge> = {
  image:   { label: 'IMG', colorKey: 'done' },
  audio:   { label: 'AUD', colorKey: 'pending' },
  diagram: { label: 'DGM', colorKey: 'warning' },
  text:    { label: 'TXT', colorKey: 'done' },
};

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function FileOutput({
  fileType,
  description,
  filePath,
  isFocused,
}: FileOutputProps): React.ReactElement {
  const { colors } = useTheme();

  const badge = TYPE_BADGES[fileType];
  const badgeColor = colors.status[badge.colorKey];
  const borderColor = isFocused ? colors.border.focus : colors.border.default;

  return (
    <Box flexDirection="column" paddingLeft={1}>
      <Box>
        <Text color={badgeColor} bold>
          [{badge.label}]
        </Text>
        <Text color={colors.text.primary}> {description}</Text>
      </Box>
      <Box paddingLeft={6}>
        <Text color={colors.text.accent} underline>
          {filePath}
        </Text>
      </Box>
    </Box>
  );
}
