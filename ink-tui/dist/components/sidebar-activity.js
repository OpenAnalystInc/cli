import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
import { Box, Text } from 'ink';
import { providerPreferences } from '../utils/provider-preferences.js';
import { PROVIDER_CONFIG } from '../utils/credential-manager.js';
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function formatTokens(tokens) {
    if (tokens < 1_000)
        return String(tokens);
    if (tokens < 1_000_000)
        return `${(tokens / 1_000).toFixed(1)}k`;
    return `${(tokens / 1_000_000).toFixed(1)}M`;
}
function formatElapsed(secs) {
    if (secs < 60)
        return `${secs}s`;
    const m = Math.floor(secs / 60);
    const s = secs % 60;
    return `${m}m ${String(s).padStart(2, '0')}s`;
}
function getPermissionDisplay(mode) {
    switch (mode) {
        case 'read-only':
            return { icon: 'R', color: '#0088FF', label: 'read-only' };
        case 'workspace-write':
            return { icon: 'W', color: '#FFD700', label: 'workspace' };
        case 'prompt':
        case undefined:
            return { icon: 'P', color: '#00BFFF', label: 'prompt' };
        case 'danger-full-access':
            return { icon: 'F', color: '#FF4444', label: 'full-access' };
        default:
            return { icon: '?', color: '#888888', label: mode || 'unknown' };
    }
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function SidebarActivity({ activity, isFocused, colors, permissionMode, elapsedSecs = 0, totalTokens = 0, }) {
    const textColor = isFocused ? colors.sidebar.itemSelected : colors.text.primary;
    const perm = getPermissionDisplay(permissionMode);
    return (_jsxs(Box, { flexDirection: "column", children: [_jsxs(Box, { children: [_jsxs(Text, { color: "#0088FF", children: [" ", '\u21C5', " "] }), _jsxs(Text, { color: textColor, children: [activity.toolCallCount, " tool calls"] })] }), _jsxs(Box, { children: [_jsxs(Text, { color: "#00CC44", children: [" ", '\u2193', " "] }), _jsxs(Text, { color: textColor, children: [formatTokens(totalTokens), " tokens"] })] }), _jsxs(Box, { children: [_jsxs(Text, { color: "#FFD700", children: [" ", '\u2299', " "] }), _jsxs(Text, { color: textColor, children: [formatElapsed(elapsedSecs), " elapsed"] })] }), _jsxs(Box, { children: [_jsxs(Text, { color: perm.color, children: [" ", perm.icon, " "] }), _jsxs(Text, { color: textColor, children: ["mode: ", perm.label] })] }), (() => {
                const dp = providerPreferences.getDefaultProvider();
                const dpConfig = dp ? PROVIDER_CONFIG[dp] : null;
                if (dpConfig) {
                    return (_jsxs(Box, { children: [_jsxs(Text, { color: "#FFD700", children: [' \u2605', " "] }), _jsx(Text, { color: textColor, children: dpConfig.displayName })] }));
                }
                return null;
            })(), activity.creditBalance != null && (_jsxs(Box, { children: [_jsx(Text, { color: colors.text.secondary, children: " $" }), _jsxs(Text, { color: textColor, children: [' ', activity.creditBalance] })] }))] }));
}
//# sourceMappingURL=sidebar-activity.js.map