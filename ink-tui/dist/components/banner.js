import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * Banner -- startup banner rendered once at the top of the chat panel.
 *
 * Mirrors the Rust `crates/tui/src/banner.rs`:
 *   - Dual-column layout: left (welcome + OA logo + account info) + right (tips + recent activity)
 *   - Rounded-corner box-drawing border in OA brand blue
 *   - Large "OA" ASCII art logo in ORANGE
 *   - Adapts to terminal width
 *
 * This component is rendered once from the chat panel when the engine
 * sends a `banner` event. It stays pinned at the top of the scroll buffer.
 */
import React, { useMemo } from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
// ---------------------------------------------------------------------------
// OA ASCII art logo (matches Rust banner.rs)
// ---------------------------------------------------------------------------
const OA_LOGO = [
    '   \u2588\u2588\u2588\u2588\u2588\u2588\u2588\u2588   \u2588\u2588\u2588\u2588         ',
    '   \u2588\u2588    \u2588\u2588  \u2588\u2588  \u2588\u2588        ',
    '   \u2588\u2588    \u2588\u2588  \u2588\u2588\u2588\u2588\u2588\u2588        ',
    '   \u2588\u2588    \u2588\u2588  \u2588\u2588  \u2588\u2588        ',
    '   \u2588\u2588\u2588\u2588\u2588\u2588\u2588\u2588  \u2588\u2588  \u2588\u2588        ',
];
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
/** Truncate a string with ellipsis if it exceeds maxLen. */
function truncate(str, maxLen) {
    if (str.length <= maxLen)
        return str;
    return str.slice(0, maxLen - 1) + '\u2026';
}
/** Truncate working directory from the left with ... prefix. */
function truncateCwd(cwd, maxLen) {
    if (cwd.length <= maxLen)
        return cwd;
    const keep = maxLen - 2;
    return '\u2026' + cwd.slice(cwd.length - keep);
}
/** Pad a string to exactly `len` characters (right-padded with spaces). */
function padRight(str, len) {
    if (str.length >= len)
        return str.slice(0, len);
    return str + ' '.repeat(len - str.length);
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function Banner({ version, username, email, org, workingDir, provider, modelDisplay, credits, tips, terminalWidth = 80, }) {
    const { colors } = useTheme();
    // Compute column widths based on terminal width.
    const layout = useMemo(() => {
        const rightW = Math.min(38, Math.max(24, Math.floor(terminalWidth * 0.35)));
        // Total inner = terminalWidth - 3 (for border chars) - 1 (middle divider)
        const totalInner = terminalWidth - 4;
        const leftW = totalInner - rightW;
        return { leftW, rightW };
    }, [terminalWidth]);
    const { leftW, rightW } = layout;
    const brandColor = colors.text.accent;
    const dimColor = colors.text.secondary;
    const headingColor = colors.text.heading;
    const doneColor = colors.status.done;
    const logoColor = '#FF8C00'; // Orange for OA logo (matches Rust: Color::Rgb(255, 140, 0))
    // Title text
    const titleText = provider
        ? `OpenAnalyst CLI v${version} \u00B7 ${provider}`
        : `OpenAnalyst CLI v${version}`;
    // -- Build rows --
    // Helper: build a branded dual-column row
    const buildRow = (leftText, leftColor, leftBold, rightText, rightColor) => {
        const lPadded = padRight(leftText, leftW);
        const rPadded = padRight(rightText, rightW);
        return (_jsxs(Text, { children: [_jsx(Text, { color: brandColor, children: '\u2502' }), _jsx(Text, { color: leftColor, bold: leftBold, children: lPadded }), _jsx(Text, { color: brandColor, children: '\u2502' }), _jsx(Text, { color: rightColor, children: rPadded }), _jsx(Text, { color: brandColor, children: '\u2502' })] }));
    };
    // Helper: build a row with the OA logo on the left (orange) and tip on the right
    const buildLogoRow = (logoLine, rightText, rightColor) => {
        const lPadded = padRight(logoLine, leftW);
        const rPadded = padRight(rightText, rightW);
        return (_jsxs(Text, { children: [_jsx(Text, { color: brandColor, children: '\u2502' }), _jsx(Text, { color: logoColor, children: lPadded }), _jsx(Text, { color: brandColor, children: '\u2502' }), _jsx(Text, { color: rightColor, children: rPadded }), _jsx(Text, { color: brandColor, children: '\u2502' })] }));
    };
    // -- Top border --
    const verText = ` ${truncate(titleText, leftW - 2)} `;
    const leftPad = Math.max(0, leftW - verText.length - 1);
    const rightTitleText = ' Tips for getting started ';
    const rightPad = Math.max(0, rightW - rightTitleText.length);
    // -- Tip lines for right column (paired with logo rows) --
    const isOA = !provider || provider === 'OpenAnalyst Inc';
    const tipContent = isOA
        ? [
            ' Run /init to create an',
            ' OPENANALYST.md file with',
            ' instructions for OpenAnalyst',
            ' Recent activity',
            ' No recent activity',
        ]
        : [
            ' Run /init to create a',
            ' project config file with',
            ' instructions for the agent',
            ' Recent activity',
            ' No recent activity',
        ];
    // -- Info lines for below the logo --
    const infoLines = [];
    // Model + provider
    if (modelDisplay) {
        const modelLine = provider
            ? ` ${modelDisplay} \u00B7 ${provider}`
            : ` ${modelDisplay}`;
        infoLines.push({ text: truncate(modelLine, leftW), color: headingColor, bold: false });
    }
    // Email + org
    if (email) {
        let line = ` ${email}`;
        if (org)
            line += ` \u00B7 ${org}`;
        infoLines.push({ text: truncate(line, leftW), color: dimColor, bold: false });
    }
    // Credits
    if (credits) {
        infoLines.push({ text: ` Credits: ${credits}`, color: doneColor, bold: false });
    }
    // Working directory
    infoLines.push({
        text: ` ${truncateCwd(workingDir, leftW - 2)}`,
        color: dimColor,
        bold: false,
    });
    return (_jsxs(Box, { flexDirection: "column", children: [_jsxs(Text, { color: brandColor, bold: true, children: ['\u256D\u2500', verText, '\u2500'.repeat(leftPad), '\u252C\u2500', rightTitleText, '\u2500'.repeat(rightPad), '\u256E'] }), buildRow(`  Welcome back, ${truncate(username, leftW - 18)}!`, headingColor, true, '', dimColor), buildRow('', dimColor, false, '', dimColor), OA_LOGO.map((logoLine, i) => {
                const tipLine = tipContent[i] ?? '';
                // "Recent activity" label is green, others are dim
                const tipColor = i === 3 ? doneColor : dimColor;
                return (_jsx(React.Fragment, { children: buildLogoRow(logoLine, tipLine, tipColor) }, `logo-${i}`));
            }), buildRow('', dimColor, false, '', dimColor), infoLines.map((info, i) => (_jsx(React.Fragment, { children: buildRow(info.text, info.color, info.bold, '', dimColor) }, `info-${i}`))), _jsxs(Text, { color: brandColor, bold: true, children: ['\u2570', '\u2500'.repeat(leftW), '\u2534', '\u2500'.repeat(rightW), '\u256F'] }), _jsx(Text, { children: ' ' }), _jsxs(Text, { children: [_jsx(Text, { color: brandColor, children: "  /help" }), _jsxs(Text, { color: dimColor, children: [" for commands ", '\u00B7', " "] }), _jsx(Text, { color: brandColor, children: "/model" }), _jsxs(Text, { color: dimColor, children: [" to switch ", '\u00B7', " "] }), _jsx(Text, { color: brandColor, children: "ctrl+c" }), _jsx(Text, { color: dimColor, children: " to exit" })] }), _jsx(Text, { children: ' ' })] }));
}
//# sourceMappingURL=banner.js.map