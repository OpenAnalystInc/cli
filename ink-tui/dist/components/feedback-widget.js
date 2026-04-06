import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * FeedbackWidget — inline feedback prompt rendered below a KnowledgeCard.
 *
 * Mirrors the Rust tui-widgets/feedback_dialog.rs:
 *   Was this helpful?  [Y] [N] [Esc dismiss]
 *
 * Selected button has bold text + background color.
 * Unselected buttons have plain colored text.
 *
 * Keybinding: y/n/Esc in scroll mode (connected via useKeypress).
 * All colors from useTheme() semantic tokens.
 */
import { useCallback } from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
import { useKeypress } from '../hooks/use-keypress.js';
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function FeedbackWidget({ queryId, selectedIndex, onSelect, onSelectionChange, isActive, }) {
    const { colors } = useTheme();
    // Handle key presses when active.
    useKeypress(useCallback((ch, key, _command) => {
        if (!isActive)
            return false;
        // Direct shortcuts.
        if (ch === 'y' || ch === 'Y') {
            onSelect('positive');
            return true;
        }
        if (ch === 'n' || ch === 'N') {
            onSelect('negative');
            return true;
        }
        if (key.escape) {
            onSelect('dismiss');
            return true;
        }
        // Tab/arrow cycling.
        if (key.tab || key.rightArrow) {
            onSelectionChange((selectedIndex + 1) % 3);
            return true;
        }
        if (key.leftArrow) {
            onSelectionChange(selectedIndex === 0 ? 2 : selectedIndex - 1);
            return true;
        }
        // Enter confirms current selection.
        if (key.return) {
            const ratings = ['positive', 'negative', 'dismiss'];
            onSelect(ratings[selectedIndex] ?? 'dismiss');
            return true;
        }
        return false;
    }, [isActive, selectedIndex, onSelect, onSelectionChange]), { isActive, priority: 60 });
    // Button styles — selected gets background + bold, unselected is plain text.
    const positiveStyle = selectedIndex === 0
        ? { color: '#000000', backgroundColor: colors.status.done, bold: true }
        : { color: colors.status.done, bold: false };
    const negativeStyle = selectedIndex === 1
        ? { color: '#000000', backgroundColor: colors.status.error, bold: true }
        : { color: colors.status.error, bold: false };
    const dismissStyle = selectedIndex === 2
        ? { color: '#000000', backgroundColor: colors.text.secondary, bold: true }
        : { color: colors.text.secondary, bold: false };
    return (_jsxs(Box, { paddingLeft: 2, children: [_jsx(Text, { color: colors.text.primary, children: "Was this helpful? " }), _jsx(Text, { color: positiveStyle.color, backgroundColor: positiveStyle.backgroundColor, bold: positiveStyle.bold, children: ' Y ' }), _jsx(Text, { children: " " }), _jsx(Text, { color: negativeStyle.color, backgroundColor: negativeStyle.backgroundColor, bold: negativeStyle.bold, children: ' N ' }), _jsx(Text, { children: " " }), _jsx(Text, { color: dismissStyle.color, backgroundColor: dismissStyle.backgroundColor, bold: dismissStyle.bold, children: ' Esc ' }), _jsxs(Text, { color: colors.text.secondary, dimColor: true, children: [' ', '·', " /feedback for corrections"] })] }));
}
//# sourceMappingURL=feedback-widget.js.map