import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
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
// Component
// ---------------------------------------------------------------------------
export const UserMessage = React.memo(function UserMessage({ text, isSlashCommand, isFocused, }) {
    const { colors } = useTheme();
    const promptColor = isSlashCommand
        ? colors.text.slashCommand
        : colors.text.userPrompt;
    const textColor = isSlashCommand
        ? colors.text.slashCommand
        : colors.text.primary;
    // Pre-process display text: collapse image/file references
    const displayText = useMemo(() => {
        if (!text)
            return text;
        return text;
    }, [text]);
    return (_jsxs(Box, { flexDirection: "row", paddingLeft: 0, marginTop: 1, ...(isFocused ? {} : {}), children: [_jsx(Box, { width: 2, flexShrink: 0, children: _jsx(Text, { color: promptColor, bold: true, children: '\u276F ' }) }), _jsx(Box, { flexGrow: 1, children: _jsx(Text, { color: textColor, bold: true, wrap: "wrap", children: displayText }) })] }));
});
//# sourceMappingURL=user-message.js.map