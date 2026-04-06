import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
/** Extract the filename from a full path. */
function basename(filePath) {
    const parts = filePath.split(/[/\\]/);
    return parts[parts.length - 1] ?? filePath;
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function ContextFileTags({ files, maxWidth, }) {
    const { colors } = useTheme();
    if (files.length === 0)
        return null;
    // Calculate which files fit within maxWidth
    const badges = [];
    let usedWidth = 0;
    let shownCount = 0;
    for (const filePath of files) {
        const name = basename(filePath);
        // Badge format: " @filename " + 1 space separator
        const badgeWidth = name.length + 3 + 1; // " @" + name + " " + separator
        // Reserve space for "+N more" indicator
        const remaining = files.length - shownCount;
        const moreIndicatorWidth = remaining > 1 ? ` +${remaining - 1} more `.length + 1 : 0;
        if (usedWidth + badgeWidth + moreIndicatorWidth > maxWidth && shownCount > 0) {
            break;
        }
        badges.push({ label: `@${name}`, key: filePath });
        usedWidth += badgeWidth;
        shownCount++;
    }
    const hiddenCount = files.length - shownCount;
    return (_jsxs(Box, { gap: 1, children: [badges.map(({ label, key }) => (_jsx(Text, { color: colors.text.accent, backgroundColor: colors.background.badge.contextFile, children: ` ${label} ` }, key))), hiddenCount > 0 && (_jsx(Text, { color: colors.text.secondary, dimColor: true, children: ` +${hiddenCount} more ` }))] }));
}
//# sourceMappingURL=context-file-tags.js.map