import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
/**
 * Autocomplete — dropdown popup for `/` slash-command completion.
 *
 * Appears below the input when the user types `/`. Shows a filterable
 * list of commands with descriptions. Supports keyboard navigation.
 *
 * Keybinding priority: 7 (above input at 5, below dialogs at 9).
 *
 * Visual design:
 *   - Max 12 visible items, scrollable
 *   - Selected item: bold with accent background
 *   - Unselected items: dim
 *   - Each item shows: command name + description
 */
import { useMemo } from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
const DEFAULT_MAX_VISIBLE = 12;
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function Autocomplete({ items, selectedIndex, visible, maxVisible = DEFAULT_MAX_VISIBLE, }) {
    const { colors } = useTheme();
    // Compute the visible window (scroll to keep selected item in view).
    const { visibleItems, startIndex } = useMemo(() => {
        if (items.length <= maxVisible) {
            return { visibleItems: items, startIndex: 0 };
        }
        // Keep the selected item roughly centered, clamped to bounds.
        let start = selectedIndex - Math.floor(maxVisible / 2);
        start = Math.max(0, start);
        start = Math.min(items.length - maxVisible, start);
        return {
            visibleItems: items.slice(start, start + maxVisible),
            startIndex: start,
        };
    }, [items, selectedIndex, maxVisible]);
    if (!visible || items.length === 0) {
        return null;
    }
    const hasScrollUp = startIndex > 0;
    const hasScrollDown = startIndex + maxVisible < items.length;
    return (_jsxs(Box, { flexDirection: "column", borderStyle: "single", borderColor: colors.border.focus, paddingX: 1, children: [hasScrollUp && (_jsxs(Text, { color: colors.text.secondary, children: ["  \u2191 ", startIndex, " more"] })), visibleItems.map((item, i) => {
                const globalIndex = startIndex + i;
                const isSelected = globalIndex === selectedIndex;
                return (_jsx(Box, { children: isSelected ? (_jsxs(Text, { backgroundColor: colors.background.focus, color: colors.text.accent, bold: true, children: ['▸ ', item.name, _jsxs(Text, { color: colors.text.secondary, children: [" \u2014 ", item.description] })] })) : (_jsxs(Text, { color: colors.text.secondary, children: ['  ', item.name, _jsxs(Text, { color: colors.text.secondary, dimColor: true, children: [" \u2014 ", item.description] })] })) }, item.name));
            }), hasScrollDown && (_jsxs(Text, { color: colors.text.secondary, children: ['  ', "\u2193 ", items.length - startIndex - maxVisible, " more"] })), _jsx(Box, { marginTop: 0, children: _jsx(Text, { color: colors.text.secondary, dimColor: true, children: "\u2191\u2193 navigate \u00B7 Tab accept \u00B7 Esc dismiss" }) })] }));
}
//# sourceMappingURL=autocomplete.js.map