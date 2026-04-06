import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
/** Truncate a string to maxWidth, appending ellipsis if needed. */
function truncate(s, maxWidth) {
    if (s.length <= maxWidth)
        return s;
    if (maxWidth > 3)
        return s.slice(0, maxWidth - 3) + '...';
    return s.slice(0, maxWidth);
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function DiffView({ filePath, added, removed, hunks, maxLines = 20, maxWidth, }) {
    const { colors } = useTheme();
    // Flatten all lines from all hunks, inserting hunk headers.
    const flatLines = [];
    for (const hunk of hunks) {
        flatLines.push({
            type: 'header',
            text: `@@ -${hunk.oldStart} +${hunk.newStart} @@`,
        });
        for (const line of hunk.lines) {
            flatLines.push({
                type: line.kind,
                text: line.kind === 'added'
                    ? `+ ${line.text}`
                    : line.kind === 'removed'
                        ? `- ${line.text}`
                        : `  ${line.text}`,
            });
        }
    }
    const totalLines = flatLines.length;
    const visibleLines = flatLines.slice(0, maxLines);
    const overflowCount = totalLines - maxLines;
    return (_jsxs(Box, { flexDirection: "column", children: [_jsxs(Text, { children: [_jsx(Text, { color: colors.text.accent, bold: true, children: filePath }), _jsx(Text, { color: colors.text.secondary, children: " " }), _jsxs(Text, { color: colors.diff.added, bold: true, children: ["+", added] }), _jsx(Text, { color: colors.text.secondary, children: " / " }), _jsxs(Text, { color: colors.diff.removed, bold: true, children: ["-", removed] })] }), visibleLines.map((line, i) => {
                const displayText = maxWidth ? truncate(line.text, maxWidth) : line.text;
                if (line.type === 'header') {
                    return (_jsx(Text, { color: colors.text.accent, dimColor: true, children: displayText }, i));
                }
                if (line.type === 'added') {
                    return (_jsx(Text, { color: colors.diff.added, children: displayText }, i));
                }
                if (line.type === 'removed') {
                    return (_jsx(Text, { color: colors.diff.removed, children: displayText }, i));
                }
                // Context line
                return (_jsx(Text, { color: colors.text.secondary, children: displayText }, i));
            }), overflowCount > 0 && (_jsxs(Text, { color: colors.text.secondary, dimColor: true, children: ["... (", overflowCount, " more lines)"] }))] }));
}
//# sourceMappingURL=diff-view.js.map