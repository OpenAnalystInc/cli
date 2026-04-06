/**
 * UserMessage — renders a single user prompt in the chat panel.
 *
 * - Cyan `>` prompt icon for normal messages
 * - Orange `>` for /slash commands
 * - Bold text for the user's input
 */

import React, { useMemo } from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface UserMessageProps {
  /** The user's input text. */
  text: string;
  /** Whether this is a /slash command (changes prompt color). */
  isSlashCommand: boolean;
  /** Whether this message is currently focused in scroll mode. */
  isFocused?: boolean;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export const UserMessage = React.memo(function UserMessage({
  text,
  isSlashCommand,
  isFocused,
}: UserMessageProps): React.ReactElement {
  const { colors } = useTheme();

  const promptColor = isSlashCommand
    ? colors.text.slashCommand
    : colors.text.userPrompt;

  const textColor = isSlashCommand
    ? colors.text.slashCommand
    : colors.text.primary;

  // Pre-process display text: collapse image/file references
  const displayText = useMemo(() => {
    if (!text) return text;
    return text;
  }, [text]);

  return (
    <Box
      flexDirection="row"
      paddingLeft={0}
      marginTop={1}
      {...(isFocused ? { } : {})}
    >
      <Box width={2} flexShrink={0}>
        <Text color={promptColor} bold>
          {'\u276F '}
        </Text>
      </Box>
      <Box flexGrow={1}>
        <Text color={textColor} bold wrap="wrap">
          {displayText}
        </Text>
      </Box>
    </Box>
  );
});
