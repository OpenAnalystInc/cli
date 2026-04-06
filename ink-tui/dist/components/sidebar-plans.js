import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Box, Text } from 'ink';
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
const MAX_NAME_WIDTH = 14;
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function statusLabel(status) {
    switch (status) {
        case 'todo': return '[TODO]';
        case 'in_progress': return '[WIP]';
        case 'done': return '[DONE]';
    }
}
function statusColor(status, colors) {
    switch (status) {
        case 'todo': return colors.text.secondary;
        case 'in_progress': return colors.status.warning;
        case 'done': return colors.status.done;
    }
}
function truncate(text, maxLen) {
    if (text.length <= maxLen)
        return text;
    return text.slice(0, maxLen - 1) + '…';
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function SidebarPlans({ plans, selectedIndex, isFocused, colors, }) {
    if (plans.length === 0) {
        return (_jsx(Text, { color: colors.text.secondary, children: "  (no plans)" }));
    }
    return (_jsx(Box, { flexDirection: "column", children: plans.map((plan, i) => {
            const isSelected = isFocused && i === selectedIndex;
            const label = statusLabel(plan.status);
            const labelColor = statusColor(plan.status, colors);
            const name = truncate(plan.name, MAX_NAME_WIDTH);
            return (_jsxs(Box, { children: [_jsxs(Text, { color: labelColor, children: [" ", label, " "] }), _jsx(Text, { color: isSelected ? colors.sidebar.itemSelected : colors.sidebar.itemDefault, bold: isSelected, children: name })] }, plan.name));
        }) }));
}
//# sourceMappingURL=sidebar-plans.js.map