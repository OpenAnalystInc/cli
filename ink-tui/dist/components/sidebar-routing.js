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
/** Category-specific colors matching Ratatui sidebar */
const CATEGORY_COLORS = {
    explore: '#00BFFF', // cyan
    research: '#FFD700', // yellow
    code: '#00FF7F', // green
    write: '#FFA500', // orange
};
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
function tierDotColor(tier) {
    switch (tier) {
        case 'fast': return '#00BFFF'; // cyan
        case 'balanced': return '#FFD700'; // yellow
        case 'capable': return '#00FF7F'; // green
        default: return '#888888';
    }
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function SidebarRouting({ routing, selectedIndex, isFocused, colors, }) {
    // Resolve default provider for display
    const defaultProvider = providerPreferences.getDefaultProvider();
    const defaultConfig = defaultProvider ? providerPreferences.getDefaultModelForProvider(defaultProvider) : null;
    return (_jsxs(Box, { flexDirection: "column", children: [defaultProvider && defaultConfig && (_jsxs(Box, { children: [_jsx(Text, { color: "#FFD700", children: ' \u2605 ' }), _jsx(Text, { color: colors.text.secondary, dimColor: true, children: truncate(defaultConfig.name, MAX_MODEL_WIDTH) })] })), CATEGORIES.map((cat, i) => {
                const entry = routing[cat];
                const isSelected = isFocused && i === selectedIndex;
                const label = CATEGORY_LABELS[cat];
                // If model name is empty, try to resolve from saved preferences or default provider
                let modelDisplay = entry.model || entry.tier;
                if (!modelDisplay && defaultConfig) {
                    modelDisplay = defaultConfig.name;
                }
                const model = truncate(modelDisplay || 'beta', MAX_MODEL_WIDTH);
                const catColor = CATEGORY_COLORS[cat];
                const dotColor = tierDotColor(entry.tier || defaultConfig?.tier || 'balanced');
                const selPrefix = isSelected ? '\u25B8' : ' ';
                const bg = isSelected ? '#333333' : undefined;
                return (_jsxs(Box, { children: [_jsx(Text, { color: "#FFD700", backgroundColor: bg, children: selPrefix }), _jsx(Text, { color: catColor, backgroundColor: bg, children: label }), _jsxs(Text, { color: dotColor, backgroundColor: bg, children: ['\u25CF', " "] }), _jsx(Text, { color: colors.text.secondary, backgroundColor: bg, children: model })] }, cat));
            })] }));
}
//# sourceMappingURL=sidebar-routing.js.map