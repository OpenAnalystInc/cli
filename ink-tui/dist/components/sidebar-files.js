import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Box, Text } from 'ink';
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
/** Max display width for filename. */
const MAX_TEXT_WIDTH = 20;
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function actionIcon(action) {
    switch (action) {
        case 'read': return '○';
        case 'edited': return '●';
        case 'created': return '+';
    }
}
function actionColor(action, colors) {
    switch (action) {
        case 'read': return colors.sidebar.fileRead;
        case 'edited': return colors.sidebar.fileEdited;
        case 'created': return colors.sidebar.fileCreated;
    }
}
function truncate(text, maxLen) {
    if (text.length <= maxLen)
        return text;
    return text.slice(0, maxLen - 1) + '…';
}
/** Extract just the filename from a path. */
function basename(filePath) {
    const sep = filePath.lastIndexOf('/');
    const bsep = filePath.lastIndexOf('\\');
    const lastSep = Math.max(sep, bsep);
    return lastSep >= 0 ? filePath.slice(lastSep + 1) : filePath;
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function SidebarFiles({ files, selectedIndex, isFocused, colors, }) {
    if (files.length === 0) {
        return (_jsx(Text, { color: colors.text.secondary, children: "  (no files yet)" }));
    }
    return (_jsx(Box, { flexDirection: "column", children: files.map((file, i) => {
            const isSelected = isFocused && i === selectedIndex;
            const icon = actionIcon(file.action);
            const iconColor = actionColor(file.action, colors);
            const name = truncate(basename(file.path), MAX_TEXT_WIDTH);
            return (_jsxs(Box, { children: [_jsxs(Text, { color: iconColor, children: [" ", icon, " "] }), _jsx(Text, { color: isSelected ? colors.sidebar.itemSelected : colors.sidebar.itemDefault, bold: isSelected, children: name })] }, file.path));
        }) }));
}
//# sourceMappingURL=sidebar-files.js.map