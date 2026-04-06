/**
 * SystemMessage — renders info, warning, and error notices in the chat.
 *
 * - Gray bullet + dim text for info
 * - Yellow bullet + yellow text for warnings
 * - Red bullet + red text for errors
 *
 * Agent lifecycle noise (spawned/completed) is filtered unless it's an error.
 */

import React from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
import type { SystemLevel } from '../types/messages.js';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface SystemMessageProps {
  text: string;
  level: SystemLevel;
  isFocused?: boolean;
}

// ---------------------------------------------------------------------------
// Noise filter — suppress agent lifecycle messages unless error
// ---------------------------------------------------------------------------

const LIFECYCLE_PATTERNS = [
  /^Agent \S+ spawned$/,
  /^Agent \S+ completed$/,
  /^Agent \S+ status changed to (Running|Pending|Completed)$/,
];

export function isLifecycleNoise(text: string, level: SystemLevel): boolean {
  if (level === 'error') return false;
  return LIFECYCLE_PATTERNS.some((pattern) => pattern.test(text));
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export const SystemMessage = React.memo(function SystemMessage({
  text,
  level,
  isFocused,
}: SystemMessageProps): React.ReactElement | null {
  const { colors } = useTheme();

  // Filter lifecycle noise
  if (isLifecycleNoise(text, level)) {
    return null;
  }

  let bulletColor: string;
  let textColor: string;
  let bullet: string;

  switch (level) {
    case 'error':
      bulletColor = colors.status.error;
      textColor = colors.status.error;
      bullet = '\u2717'; // ✗
      break;
    case 'warning':
      bulletColor = colors.status.warning;
      textColor = colors.status.warning;
      bullet = '\u26A0'; // ⚠
      break;
    case 'info':
    default:
      bulletColor = colors.text.secondary;
      textColor = colors.text.secondary;
      bullet = '\u2139'; // ℹ
      break;
  }

  return (
    <Box
      flexDirection="row"
      paddingLeft={0}
      marginTop={0}
      {...(isFocused ? {} : {})}
    >
      <Box width={2} flexShrink={0}>
        <Text color={bulletColor}>{bullet} </Text>
      </Box>
      <Box flexGrow={1}>
        <Text
          color={textColor}
          dimColor={level === 'info'}
          wrap="wrap"
        >
          {text}
        </Text>
      </Box>
    </Box>
  );
});
