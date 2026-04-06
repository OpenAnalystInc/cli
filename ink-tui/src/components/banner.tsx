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

const OA_LOGO: readonly string[] = [
  '\u2588\u2588\u2588\u2588\u2588\u2588\u2588\u2588   \u2588\u2588\u2588\u2588',
  '\u2588\u2588    \u2588\u2588  \u2588\u2588  \u2588\u2588',
  '\u2588\u2588    \u2588\u2588  \u2588\u2588\u2588\u2588\u2588\u2588',
  '\u2588\u2588    \u2588\u2588  \u2588\u2588  \u2588\u2588',
  '\u2588\u2588\u2588\u2588\u2588\u2588\u2588\u2588  \u2588\u2588  \u2588\u2588',
];

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface BannerProps {
  version: string;
  username: string;
  email?: string;
  org?: string;
  workingDir: string;
  provider?: string;
  modelDisplay?: string;
  credits?: string;
  tips: string[];
  terminalWidth?: number;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function truncate(str: string, maxLen: number): string {
  if (str.length <= maxLen) return str;
  return str.slice(0, maxLen - 1) + '\u2026';
}

function truncateCwd(cwd: string, maxLen: number): string {
  if (cwd.length <= maxLen) return cwd;
  const keep = maxLen - 2;
  return '\u2026' + cwd.slice(cwd.length - keep);
}

function padRight(str: string, len: number): string {
  if (str.length >= len) return str.slice(0, len);
  return str + ' '.repeat(len - str.length);
}

function centerPad(str: string, width: number): string {
  if (str.length >= width) return str.slice(0, width);
  const lp = Math.floor((width - str.length) / 2);
  const rp = width - str.length - lp;
  return ' '.repeat(lp) + str + ' '.repeat(rp);
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function Banner({
  version,
  username,
  email,
  org,
  workingDir,
  provider,
  modelDisplay,
  credits,
  tips,
  terminalWidth = 80,
}: BannerProps): React.ReactElement {
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
  const tipLines: Array<{ text: string; color: string }> = [];

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
  } else {
    tipLines.push({ text: ' Run /init to create a', color: dimColor });
    tipLines.push({ text: ' project config file with', color: dimColor });
    tipLines.push({ text: ' instructions for the agent', color: dimColor });
    tipLines.push({ text: ' Recent activity', color: doneColor });
    tipLines.push({ text: ' No recent activity', color: dimColor });
  }

  // ── Row builder: all pure <Text> ──
  const row = (
    leftText: string,
    leftColor: string,
    leftBold: boolean,
    rightText: string,
    rightColor: string,
  ): React.ReactElement => {
    const lPad = padRight(leftText, leftW);
    const rPad = padRight(rightText, rightW);
    return (
      <Text>
        <Text color={brandColor}>{'\u2502'}</Text>
        <Text color={leftColor} bold={leftBold}>{lPad}</Text>
        <Text color={brandColor}>{'\u2502'}</Text>
        <Text color={rightColor}>{rPad}</Text>
        <Text color={brandColor}>{'\u2502'}</Text>
      </Text>
    );
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

  return (
    <Box flexDirection="column">
      {/* ── Top border ── */}
      <Text color={brandColor} bold>
        {'\u256D'}{'\u2500' + verText + '\u2500'.repeat(leftFill)}{'\u252C'}{'\u2500'.repeat(rightFill)}{'\u256E'}
      </Text>

      {/* ── Welcome | Tips header ── */}
      {row(
        centerPad(welcomeText, leftW),
        colors.text.strong, true,
        ' Tips for getting started', doneColor,
      )}

      {/* ── Blank spacer ── */}
      {row('', dimColor, false, '', dimColor)}

      {/* ── OA logo (5 rows) + tip content on right ── */}
      {OA_LOGO.map((logoLine, i) => {
        const tip = tipLines[i + 2]; // offset by 2 (welcome + spacer)
        const lPad = padRight(centerPad(logoLine, leftW), leftW);
        const rPad = padRight(tip?.text ?? '', rightW);
        return (
          <Text key={`logo-${i}`}>
            <Text color={brandColor}>{'\u2502'}</Text>
            <Text color={logoColor}>{lPad}</Text>
            <Text color={brandColor}>{'\u2502'}</Text>
            <Text color={tip?.color ?? dimColor}>{rPad}</Text>
            <Text color={brandColor}>{'\u2502'}</Text>
          </Text>
        );
      })}

      {/* ── Blank separator ── */}
      {row('', dimColor, false, '', dimColor)}

      {/* ── Info section: 2-column labeled layout ── */}
      {(() => {
        // Build labeled info rows: "  Label:  value"
        // Split into left-label and right-value within leftW
        const infoRow = (
          label: string,
          value: string,
          labelColor: string,
          valueColor: string,
          tip: string,
          tipColor: string,
        ): React.ReactElement => {
          const labelStr = `  ${label}  `;
          const valueStr = truncate(value, leftW - labelStr.length - 1);
          const combined = labelStr + valueStr;
          const lPad = padRight(combined, leftW);
          const rPad = padRight(tip, rightW);
          return (
            <Text>
              <Text color={brandColor}>{'\u2502'}</Text>
              <Text color={labelColor} bold>{labelStr}</Text>
              <Text color={valueColor}>{padRight(valueStr, leftW - labelStr.length)}</Text>
              <Text color={brandColor}>{'\u2502'}</Text>
              <Text color={tipColor}>{rPad}</Text>
              <Text color={brandColor}>{'\u2502'}</Text>
            </Text>
          );
        };

        const elements: React.ReactElement[] = [];

        // Model
        if (modelDisplay) {
          elements.push(
            <React.Fragment key="model">
              {infoRow('Model:', modelDisplay, dimColor, colors.text.primary, '', dimColor)}
            </React.Fragment>
          );
        }

        // Provider
        if (provider) {
          elements.push(
            <React.Fragment key="provider">
              {infoRow('Provider:', provider, dimColor, colors.text.primary, '', dimColor)}
            </React.Fragment>
          );
        }

        // Credits — show balance or status clearly
        const creditDisplay = !credits || credits === 'No API key configured'
          ? 'Not configured'
          : credits === 'Connected'
            ? 'Active (usage-based)'
            : credits;
        const creditColor = credits && credits.startsWith('$') ? doneColor : dimColor;
        elements.push(
          <React.Fragment key="credits">
            {infoRow('Credits:', creditDisplay, dimColor, creditColor, '', dimColor)}
          </React.Fragment>
        );

        // Working directory
        elements.push(
          <React.Fragment key="cwd">
            {infoRow('Dir:', cwdLine, dimColor, dimColor, '', dimColor)}
          </React.Fragment>
        );

        return elements;
      })()}

      {/* ── Bottom border ── */}
      <Text color={brandColor} bold>
        {'\u2570'}{'\u2500'.repeat(leftW)}{'\u2534'}{'\u2500'.repeat(rightW)}{'\u256F'}
      </Text>

      {/* Hint line (directly after border, no gap) */}
      <Text>
        <Text color={brandColor}>  /help</Text>
        <Text color={dimColor}> for commands {'\u00B7'} </Text>
        <Text color={brandColor}>/model</Text>
        <Text color={dimColor}> to switch {'\u00B7'} </Text>
        <Text color={brandColor}>ctrl+c</Text>
        <Text color={dimColor}> to exit</Text>
      </Text>
    </Box>
  );
}
