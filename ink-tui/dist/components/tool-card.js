import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
/**
 * ToolCard — inline bordered tool call card rendered inside the chat.
 *
 * Mirrors the Rust tui-widgets/tool_card.rs widget:
 *   - Rounded border, color by status (running=brand blue, completed=dim, failed=red)
 *   - Title line: spinner/check/cross + tool name + elapsed time
 *   - Input preview (first line, truncated)
 *   - Expanded: separator + output lines (max 20, with overflow indicator)
 *   - Optional DiffView for Edit/Write tools
 *
 * All colors from useTheme() semantic tokens.
 */
import { useState, useEffect, useRef } from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
import { DiffView } from './diff-view.js';
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
/** Braille spinner frames — same as Rust spinner. */
const SPINNER_FRAMES = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
const SPINNER_INTERVAL_MS = 100;
const MAX_INPUT_CHARS = 60;
const MAX_OUTPUT_LINES = 20;
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function truncate(s, maxLen) {
    if (s.length <= maxLen)
        return s;
    if (maxLen > 3)
        return s.slice(0, maxLen - 3) + '...';
    return s.slice(0, maxLen);
}
function formatDuration(ms) {
    if (ms < 1000)
        return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
}
/** Extract first line and truncate it. */
function inputPreview(input) {
    const firstLine = input.split('\n')[0] ?? '';
    return truncate(firstLine, MAX_INPUT_CHARS);
}
// ---------------------------------------------------------------------------
// Spinner hook — animates through brand gradient frames
// ---------------------------------------------------------------------------
function useSpinner(active) {
    const [frameIndex, setFrameIndex] = useState(0);
    const { getSpinnerGradient, colors } = useTheme();
    const gradientRef = useRef(getSpinnerGradient(SPINNER_FRAMES.length));
    useEffect(() => {
        if (!active)
            return;
        const interval = setInterval(() => {
            setFrameIndex((prev) => (prev + 1) % SPINNER_FRAMES.length);
        }, SPINNER_INTERVAL_MS);
        return () => clearInterval(interval);
    }, [active]);
    return {
        frame: SPINNER_FRAMES[frameIndex] ?? '⠋',
        color: gradientRef.current[frameIndex] ?? colors.spinner.active,
    };
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function ToolCard({ toolId, toolName, status, input, output, durationMs, diff, expanded, onToggleExpand, isFocused, }) {
    const { colors } = useTheme();
    const spinner = useSpinner(status === 'running');
    // Resolve border color from semantic tokens.
    const borderColor = status === 'running'
        ? colors.toolCard.running
        : status === 'completed'
            ? colors.toolCard.completed
            : colors.toolCard.failed;
    // Slightly brighter border when focused in scroll mode.
    const effectiveBorderColor = isFocused ? colors.border.focus : borderColor;
    // Status icon.
    const statusIcon = status === 'running'
        ? { char: spinner.frame, color: spinner.color }
        : status === 'completed'
            ? { char: '\u2713', color: colors.status.done }
            : { char: '\u2717', color: colors.status.error };
    // Duration label.
    const durationLabel = durationMs != null ? formatDuration(durationMs) : '';
    // ── Compact mode ──────────────────────────────────────────────────────
    // Completed/failed tools without diffs render as a single clean line,
    // matching Claude Code's compact tool-call display style.
    const isCompact = status !== 'running' && !expanded && diff == null;
    if (isCompact) {
        return (_jsxs(Box, { paddingLeft: 2, children: [_jsxs(Text, { color: statusIcon.color, children: [statusIcon.char, " "] }), _jsx(Text, { color: colors.text.accent, bold: true, children: toolName }), _jsxs(Text, { color: colors.text.secondary, children: ["(", inputPreview(input), ")"] }), durationLabel !== '' && (_jsxs(Text, { color: colors.text.secondary, children: [" \\u2500\\u2500 ", durationLabel] }))] }));
    }
    // ── Full bordered card (running, expanded, or has diff) ───────────────
    // Expand chevron.
    const chevron = expanded ? '\u25BE' : '\u25B8';
    // Output lines for expanded view.
    const outputLines = output?.split('\n') ?? [];
    const visibleOutputLines = outputLines.slice(0, MAX_OUTPUT_LINES);
    const overflowCount = outputLines.length - MAX_OUTPUT_LINES;
    return (_jsxs(Box, { flexDirection: "column", borderStyle: "round", borderColor: effectiveBorderColor, paddingX: 1, children: [_jsxs(Box, { children: [_jsxs(Text, { color: statusIcon.color, children: [statusIcon.char, " "] }), _jsx(Text, { color: colors.text.accent, bold: true, children: toolName }), durationLabel !== '' && (_jsxs(Text, { color: colors.text.secondary, children: [" ", ' ', "\\u2500\\u2500 ", durationLabel, " "] })), _jsxs(Text, { color: effectiveBorderColor, children: [" ", chevron] })] }), _jsx(Text, { color: colors.text.primary, children: inputPreview(input) }), expanded && (_jsx(Box, { flexDirection: "column", marginTop: 1, children: diff != null ? (_jsx(DiffView, { filePath: diff.filePath, added: diff.added, removed: diff.removed, hunks: diff.hunks, maxLines: MAX_OUTPUT_LINES })) : output != null ? (_jsxs(Box, { flexDirection: "column", children: [visibleOutputLines.map((line, i) => (_jsx(Text, { color: colors.text.primary, children: line }, i))), overflowCount > 0 && (_jsxs(Text, { color: colors.text.secondary, dimColor: true, children: ["... (", overflowCount, " more lines)"] }))] })) : null }))] }));
}
//# sourceMappingURL=tool-card.js.map