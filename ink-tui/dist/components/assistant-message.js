import { jsx as _jsx } from "react/jsx-runtime";
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
// Component
// ---------------------------------------------------------------------------
export const AssistantMessage = React.memo(function AssistantMessage({ content, streaming, isFocused, }) {
    return (_jsx(Box, { flexDirection: "column", paddingLeft: 2, marginTop: 0, ...(isFocused ? {} : {}), children: _jsx(MarkdownRenderer, { content: content, isStreaming: streaming }) }));
});
//# sourceMappingURL=assistant-message.js.map