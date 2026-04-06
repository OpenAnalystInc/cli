import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Box, Text } from 'ink';
import { providerPreferences } from '../utils/provider-preferences.js';
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
/** The 4 routing categories in display order. */
const CATEGORIES = ['explore', 'research', 'code', 'write'];
/** Display labels (lowercase, padded to 10 chars for alignment). */
const CATEGORY_LABELS = {
    explore: 'explore   ',
    research: 'research  ',
    code: 'code      ',
    write: 'write     ',
};
/**
 * Category-specific colors matching Ratatui sidebar.
 * Resolved at render time from semantic tokens via getCategoryColor().
 */
/** Map categories to semantic token colors. */
function getCategoryColor(cat, colors) {
    switch (cat) {
        case 'explore': return colors.text.accent; // cyan
        case 'research': return colors.status.warning; // yellow
        case 'code': return colors.status.done; // green
        case 'write': return colors.text.slashCommand; // orange
    }
}
/** Map tier to semantic token colors. */
function getTierDotColor(tier, colors) {
    switch (tier) {
        case 'fast': return colors.text.accent; // cyan
        case 'balanced': return colors.status.warning; // yellow
        case 'capable': return colors.status.done; // green
        default: return colors.text.secondary; // dim
    }
}
/** Max model name width for 26-char sidebar. */
const MAX_MODEL_WIDTH = 10;
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function truncate(text, maxLen) {
    if (text.length <= maxLen)
        return text;
    return text.slice(0, maxLen - 1) + '\u2026';
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function SidebarRouting({ routing, selectedIndex, isFocused, colors, }) {
    // Resolve default provider for display
    const defaultProvider = providerPreferences.getDefaultProvider();
    const defaultConfig = defaultProvider ? providerPreferences.getDefaultModelForProvider(defaultProvider) : null;
    return (_jsxs(Box, { flexDirection: "column", children: [defaultProvider && defaultConfig && (_jsxs(Box, { children: [_jsx(Text, { color: colors.status.warning, children: ' \u2605 ' }), _jsx(Text, { color: colors.text.secondary, dimColor: true, children: truncate(defaultConfig.name, MAX_MODEL_WIDTH) })] })), CATEGORIES.map((cat, i) => {
                const entry = routing[cat];
                const isSelected = isFocused && i === selectedIndex;
                const label = CATEGORY_LABELS[cat];
                // If model name is empty, try to resolve from saved preferences or default provider
                let modelDisplay = entry.model || entry.tier;
                if (!modelDisplay && defaultConfig) {
                    modelDisplay = defaultConfig.name;
                }
                const model = truncate(modelDisplay || 'beta', MAX_MODEL_WIDTH);
                const catColor = getCategoryColor(cat, colors);
                const dotColor = getTierDotColor(entry.tier || defaultConfig?.tier || 'balanced', colors);
                const selPrefix = isSelected ? '\u25B8' : ' ';
                const bg = isSelected ? colors.sidebar.border : undefined;
                return (_jsxs(Box, { children: [_jsx(Text, { color: colors.status.warning, backgroundColor: bg, children: selPrefix }), _jsx(Text, { color: catColor, backgroundColor: bg, children: label }), _jsxs(Text, { color: dotColor, backgroundColor: bg, children: ['\u25CF', " "] }), _jsx(Text, { color: colors.text.secondary, backgroundColor: bg, children: model })] }, cat));
            })] }));
}
//# sourceMappingURL=sidebar-routing.js.map