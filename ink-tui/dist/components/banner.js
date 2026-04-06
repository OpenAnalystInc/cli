import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * Banner -- startup banner rendered once at the top of the chat panel.
 *
 * Matches Rust banner.rs structure exactly, with centered left-column text:
 *   - Dual-column: left (centered welcome + OA logo + account info) + right (tips + activity)
 *   - Rounded-corner box-drawing border in OA brand blue
 *   - Large "OA" ASCII art logo in ORANGE — centered
 *   - "Welcome back, ..." bright white bold — centered
 *   - model · provider — centered (white)
 *   - email — centered (dim)
 *   - Credits: ... — centered (green)
 *   - cwd — centered (dim)
 *   - Right column: "Tips for getting started" header (green) + tip lines + Recent activity
 */
import React, { useMemo } from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
// ---------------------------------------------------------------------------
// OA ASCII art logo — trimmed for accurate centering
// ---------------------------------------------------------------------------
const OA_LOGO = [
    '\u2588\u2588\u2588\u2588\u2588\u2588\u2588\u2588   \u2588\u2588\u2588\u2588',
    '\u2588\u2588    \u2588\u2588  \u2588\u2588  \u2588\u2588',
    '\u2588\u2588    \u2588\u2588  \u2588\u2588\u2588\u2588\u2588\u2588',
    '\u2588\u2588    \u2588\u2588  \u2588\u2588  \u2588\u2588',
    '\u2588\u2588\u2588\u2588\u2588\u2588\u2588\u2588  \u2588\u2588  \u2588\u2588',
];
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function truncate(str, maxLen) {
    if (str.length <= maxLen)
        return str;
    return str.slice(0, maxLen - 1) + '\u2026';
}
function truncateCwd(cwd, maxLen) {
    if (cwd.length <= maxLen)
        return cwd;
    const keep = maxLen - 2;
    return '\u2026' + cwd.slice(cwd.length - keep);
}
function padRight(str, len) {
    if (str.length >= len)
        return str.slice(0, len);
    return str + ' '.repeat(len - str.length);
}
function centerPad(str, width) {
    if (str.length >= width)
        return str.slice(0, width);
    const lp = Math.floor((width - str.length) / 2);
    const rp = width - str.length - lp;
    return ' '.repeat(lp) + str + ' '.repeat(rp);
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function Banner({ version, username, email, org, workingDir, provider, modelDisplay, credits, tips, terminalWidth = 80, }) {
    const { colors } = useTheme();
    const layout = useMemo(() => {
        // Cap banner width like Claude Code — compact, not full terminal width
        const maxBannerWidth = Math.min(terminalWidth - 2, 100);
        const totalInner = maxBannerWidth - 3; // 3 border chars: │ │ │
        const rightW = Math.min(34, Math.max(18, Math.floor(totalInner * 0.35)));
        const leftW = totalInner - rightW;
        return { leftW, rightW };
    }, [terminalWidth]);
    const { leftW, rightW } = layout;
    const brandColor = colors.text.accent;
    const dimColor = colors.text.secondary;
    const doneColor = colors.status.done;
    const logoColor = colors.text.slashCommand; // OA Orange — via semantic token
    // ── Top border: ╭─ OpenAnalyst CLI v2.0.12 ──┬──────────────╮ ──
    const titleText = provider && provider !== 'OpenAnalyst Inc'
        ? `OpenAnalyst CLI v${version} \u00B7 ${provider}`
        : `OpenAnalyst CLI v${version}`;
    const verText = ` ${truncate(titleText, leftW - 4)} `;
    const leftFill = Math.max(0, leftW - verText.length - 1);
    const rightFill = Math.max(0, rightW);
    // ── Right column content (matches Rust banner.rs tip_lines) ──
    const isOA = !provider || provider === 'OpenAnalyst Inc';
    const tipLines = [];
    // Row 0: blank (paired with Welcome row)
    tipLines.push({ text: '', color: dimColor });
    // Row 1: blank spacer
    tipLines.push({ text: '', color: dimColor });
    // Rows 2-6: paired with logo rows
    if (isOA) {
        tipLines.push({ text: ' Run /init to create an', color: dimColor });
        tipLines.push({ text: ' OPENANALYST.md file with', color: dimColor });
        tipLines.push({ text: ' instructions for OpenAnalyst', color: dimColor });
        tipLines.push({ text: ' Recent activity', color: doneColor });
        tipLines.push({ text: ' No recent activity', color: dimColor });
    }
    else {
        tipLines.push({ text: ' Run /init to create a', color: dimColor });
        tipLines.push({ text: ' project config file with', color: dimColor });
        tipLines.push({ text: ' instructions for the agent', color: dimColor });
        tipLines.push({ text: ' Recent activity', color: doneColor });
        tipLines.push({ text: ' No recent activity', color: dimColor });
    }
    // ── Row builder: all pure <Text> ──
    const row = (leftText, leftColor, leftBold, rightText, rightColor) => {
        const lPad = padRight(leftText, leftW);
        const rPad = padRight(rightText, rightW);
        return (_jsxs(Text, { children: [_jsx(Text, { color: brandColor, children: '\u2502' }), _jsx(Text, { color: leftColor, bold: leftBold, children: lPad }), _jsx(Text, { color: brandColor, children: '\u2502' }), _jsx(Text, { color: rightColor, children: rPad }), _jsx(Text, { color: brandColor, children: '\u2502' })] }));
    };
    // ── Left column content — all centered ──
    const welcomeText = `Welcome back, ${truncate(username, leftW - 20)}!`;
    const modelLine = modelDisplay
        ? (provider ? `${modelDisplay} \u00B7 ${provider}` : modelDisplay)
        : '';
    let emailLine = '';
    if (email) {
        emailLine = org ? `${email} \u00B7 ${org}` : email;
    }
    const creditsLine = credits
        ? `Credits: ${credits}`
        : 'Credits: checking\u2026';
    const cwdLine = truncateCwd(workingDir, leftW - 4);
    return (_jsxs(Box, { flexDirection: "column", children: [_jsxs(Text, { color: brandColor, bold: true, children: ['\u256D', '\u2500' + verText + '\u2500'.repeat(leftFill), '\u252C', '\u2500'.repeat(rightFill), '\u256E'] }), row(centerPad(welcomeText, leftW), colors.text.strong, true, ' Tips for getting started', doneColor), row('', dimColor, false, '', dimColor), OA_LOGO.map((logoLine, i) => {
                const tip = tipLines[i + 2]; // offset by 2 (welcome + spacer)
                const lPad = padRight(centerPad(logoLine, leftW), leftW);
                const rPad = padRight(tip?.text ?? '', rightW);
                return (_jsxs(Text, { children: [_jsx(Text, { color: brandColor, children: '\u2502' }), _jsx(Text, { color: logoColor, children: lPad }), _jsx(Text, { color: brandColor, children: '\u2502' }), _jsx(Text, { color: tip?.color ?? dimColor, children: rPad }), _jsx(Text, { color: brandColor, children: '\u2502' })] }, `logo-${i}`));
            }), row('', dimColor, false, '', dimColor), (() => {
                // Build labeled info rows: "  Label:  value"
                // Split into left-label and right-value within leftW
                const infoRow = (label, value, labelColor, valueColor, tip, tipColor) => {
                    const labelStr = `  ${label}  `;
                    const valueStr = truncate(value, leftW - labelStr.length - 1);
                    const combined = labelStr + valueStr;
                    const lPad = padRight(combined, leftW);
                    const rPad = padRight(tip, rightW);
                    return (_jsxs(Text, { children: [_jsx(Text, { color: brandColor, children: '\u2502' }), _jsx(Text, { color: labelColor, bold: true, children: labelStr }), _jsx(Text, { color: valueColor, children: padRight(valueStr, leftW - labelStr.length) }), _jsx(Text, { color: brandColor, children: '\u2502' }), _jsx(Text, { color: tipColor, children: rPad }), _jsx(Text, { color: brandColor, children: '\u2502' })] }));
                };
                const elements = [];
                // Model
                if (modelDisplay) {
                    elements.push(_jsx(React.Fragment, { children: infoRow('Model:', modelDisplay, dimColor, colors.text.primary, '', dimColor) }, "model"));
                }
                // Provider
                if (provider) {
                    elements.push(_jsx(React.Fragment, { children: infoRow('Provider:', provider, dimColor, colors.text.primary, '', dimColor) }, "provider"));
                }
                // Credits — show balance or status clearly
                const creditDisplay = !credits || credits === 'No API key configured'
                    ? 'Not configured'
                    : credits === 'Connected'
                        ? 'Active (usage-based)'
                        : credits;
                const creditColor = credits && credits.startsWith('$') ? doneColor : dimColor;
                elements.push(_jsx(React.Fragment, { children: infoRow('Credits:', creditDisplay, dimColor, creditColor, '', dimColor) }, "credits"));
                // Working directory
                elements.push(_jsx(React.Fragment, { children: infoRow('Dir:', cwdLine, dimColor, dimColor, '', dimColor) }, "cwd"));
                return elements;
            })(), _jsxs(Text, { color: brandColor, bold: true, children: ['\u2570', '\u2500'.repeat(leftW), '\u2534', '\u2500'.repeat(rightW), '\u256F'] }), _jsxs(Text, { children: [_jsx(Text, { color: brandColor, children: "  /help" }), _jsxs(Text, { color: dimColor, children: [" for commands ", '\u00B7', " "] }), _jsx(Text, { color: brandColor, children: "/model" }), _jsxs(Text, { color: dimColor, children: [" to switch ", '\u00B7', " "] }), _jsx(Text, { color: brandColor, children: "ctrl+c" }), _jsx(Text, { color: dimColor, children: " to exit" })] })] }));
}
//# sourceMappingURL=banner.js.map