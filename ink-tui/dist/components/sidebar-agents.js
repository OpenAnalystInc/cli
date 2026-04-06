import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Box, Text } from 'ink';
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
/** Max display width for agent task summary (sidebar is 26ch, minus borders/icons). */
const MAX_TEXT_WIDTH = 20;
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function statusIcon(status) {
    switch (status) {
        case 'Pending': return '◦';
        case 'Running': return '●';
        case 'Completed': return '✓';
        case 'Failed': return '✗';
    }
}
function statusColor(status, colors) {
    switch (status) {
        case 'Pending': return colors.text.secondary;
        case 'Running': return colors.status.running;
        case 'Completed': return colors.status.done;
        case 'Failed': return colors.status.error;
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
export function SidebarAgents({ agents, selectedIndex, isFocused, colors, }) {
    if (agents.length === 0) {
        return (_jsx(Text, { color: colors.text.secondary, children: "  (none active)" }));
    }
    return (_jsx(Box, { flexDirection: "column", children: agents.map((agent, i) => {
            const isSelected = isFocused && i === selectedIndex;
            const icon = statusIcon(agent.status);
            const iconColor = statusColor(agent.status, colors);
            const label = truncate(agent.taskSummary || agent.agentId, MAX_TEXT_WIDTH);
            return (_jsxs(Box, { children: [_jsxs(Text, { color: iconColor, children: [" ", icon, " "] }), _jsx(Text, { color: isSelected ? colors.sidebar.itemSelected : colors.sidebar.itemDefault, bold: isSelected, children: label })] }, agent.agentId));
        }) }));
}
//# sourceMappingURL=sidebar-agents.js.map