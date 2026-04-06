/**
 * AssistantMessage — renders an LLM response with markdown formatting.
 *
 * - 2-space left indent
 * - Full markdown rendering with syntax highlighting
 * - During streaming: shows content accumulated so far + blinking cursor
 */

import React from 'react';
import { Box } from 'ink';
import { MarkdownRenderer } from './markdown-renderer.js';

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
  return (
    <Box
      flexDirection="column"
      paddingLeft={2}
      marginTop={0}
      {...(isFocused ? {} : {})}
    >
      <MarkdownRenderer content={content} isStreaming={streaming} />
    </Box>
  );
});
