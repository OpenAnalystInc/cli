import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
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
// ---------------------------------------------------------------------------
// Noise filter — suppress agent lifecycle messages unless error
// ---------------------------------------------------------------------------
const LIFECYCLE_PATTERNS = [
    /^Agent \S+ spawned$/,
    /^Agent \S+ completed$/,
    /^Agent \S+ status changed to (Running|Pending|Completed)$/,
];
export function isLifecycleNoise(text, level) {
    if (level === 'error')
        return false;
    return LIFECYCLE_PATTERNS.some((pattern) => pattern.test(text));
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export const SystemMessage = React.memo(function SystemMessage({ text, level, isFocused, }) {
    const { colors } = useTheme();
    // Filter lifecycle noise
    if (isLifecycleNoise(text, level)) {
        return null;
    }
    let bulletColor;
    let textColor;
    let bullet;
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
    return (_jsxs(Box, { flexDirection: "row", paddingLeft: 0, marginTop: 0, ...(isFocused ? {} : {}), children: [_jsx(Box, { width: 2, flexShrink: 0, children: _jsxs(Text, { color: bulletColor, children: [bullet, " "] }) }), _jsx(Box, { flexGrow: 1, children: _jsx(Text, { color: textColor, dimColor: level === 'info', wrap: "wrap", children: text }) })] }));
});
//# sourceMappingURL=system-message.js.map