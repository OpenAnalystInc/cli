/**
 * AssistantMessage — renders an LLM response with markdown formatting.
 *
 * - 2-space left indent
 * - Full markdown rendering with syntax highlighting
 * - During streaming: shows content accumulated so far + blinking cursor
 */

import React from 'react';
import { Box, Text } from 'ink';
import { MarkdownRenderer } from './markdown-renderer.js';
import { useTheme } from '../contexts/theme-context.js';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface AssistantMessageProps {
  /** Accumulated markdown content. */
  content: string;
  /** True while the LLM is still generating tokens. */
  streaming: boolean;
  /** Whether this message is currently focused in scroll mode. */
  isFocused?: boolean;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export const AssistantMessage = React.memo(function AssistantMessage({
  content,
  streaming,
  isFocused,
}: AssistantMessageProps): React.ReactElement {
  const { colors } = useTheme();

  return (
    <Box
      flexDirection="row"
      marginTop={0}
      {...(isFocused ? {} : {})}
    >
      {/* Leading bullet — Claude Code style indicator */}
      <Box width={2} flexShrink={0}>
        <Text color={colors.text.accent}>{'\u25CF'} </Text>
      </Box>
      <Box flexDirection="column" flexGrow={1}>
        <MarkdownRenderer content={content} isStreaming={streaming} />
      </Box>
    </Box>
  );
});
