import { jsx as _jsx } from "react/jsx-runtime";
import { Text } from 'ink';
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function ModeBadge({ label, bgColor, textColor, bold = true, }) {
    return (_jsx(Text, { backgroundColor: bgColor, color: textColor, bold: bold, children: ` ${label} ` }));
}
//# sourceMappingURL=mode-badge.js.map