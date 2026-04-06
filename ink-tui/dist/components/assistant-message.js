import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
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
// Component
// ---------------------------------------------------------------------------
export const AssistantMessage = React.memo(function AssistantMessage({ content, streaming, isFocused, }) {
    const { colors } = useTheme();
    return (_jsxs(Box, { flexDirection: "row", marginTop: 0, ...(isFocused ? {} : {}), children: [_jsx(Box, { width: 2, flexShrink: 0, children: _jsxs(Text, { color: colors.text.accent, children: ['\u25CF', " "] }) }), _jsx(Box, { flexDirection: "column", flexGrow: 1, children: _jsx(MarkdownRenderer, { content: content, isStreaming: streaming }) })] }));
});
//# sourceMappingURL=assistant-message.js.map