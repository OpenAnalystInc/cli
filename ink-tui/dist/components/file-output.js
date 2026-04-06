import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
const TYPE_BADGES = {
    image: { label: 'IMG', colorKey: 'done' },
    audio: { label: 'AUD', colorKey: 'pending' },
    diagram: { label: 'DGM', colorKey: 'warning' },
    text: { label: 'TXT', colorKey: 'done' },
};
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function FileOutput({ fileType, description, filePath, isFocused, }) {
    const { colors } = useTheme();
    const badge = TYPE_BADGES[fileType];
    const badgeColor = colors.status[badge.colorKey];
    const borderColor = isFocused ? colors.border.focus : colors.border.default;
    return (_jsxs(Box, { flexDirection: "column", paddingLeft: 1, children: [_jsxs(Box, { children: [_jsxs(Text, { color: badgeColor, bold: true, children: ["[", badge.label, "]"] }), _jsxs(Text, { color: colors.text.primary, children: [" ", description] })] }), _jsx(Box, { paddingLeft: 6, children: _jsx(Text, { color: colors.text.accent, underline: true, children: filePath }) })] }));
}
//# sourceMappingURL=file-output.js.map