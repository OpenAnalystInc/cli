/**
 * StatusBar -- persistent single-line bar between chat and input.
 *
 * Matches Ratatui design:
 *
 *   Left side (when active):
 *     * Thinking... (4m 55s . down-arrow 5.0k tokens)
 *
 *   Right side (always):
 *     All keybinding hints in one line:
 *     Esc:input . Tab:section . j/k:nav . Ctrl+C:quit . Ctrl+B:bg . Ctrl+P:mode . F2:hide
 *
 * When idle/done the left side is hidden for a clean look.
 * The "done" checkmark auto-hides after 2 seconds.
 */

import React, { useState, useEffect, useRef } from 'react';
import { Box, Text } from 'ink';
import { useUIState } from '../contexts/ui-state-context.js';
import { useTheme } from '../contexts/theme-context.js';
import { OaSpinner } from './spinner.js';
import type { AgentPhase } from '../types/messages.js';
import type { AppMode } from '../contexts/ui-state-context.js';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/**
 * Format milliseconds into human-readable elapsed time.
 */
function formatElapsed(ms: number): string {
  const totalSecs = Math.floor(ms / 1000);
  if (totalSecs < 60) {
    return `${totalSecs}s`;
  }
  const minutes = Math.floor(totalSecs / 60);
  const seconds = totalSecs % 60;
  if (minutes < 60) {
    return `${minutes}m ${String(seconds).padStart(2, '0')}s`;
  }
  const hours = Math.floor(minutes / 60);
  const remainMins = minutes % 60;
  return `${hours}h ${String(remainMins).padStart(2, '0')}m`;
}

/**
 * Format token count compactly.
 */
function formatTokens(tokens: number): string {
  if (tokens < 1_000) return String(tokens);
  if (tokens < 1_000_000) return `${(tokens / 1_000).toFixed(1)}k`;
  return `${(tokens / 1_000_000).toFixed(1)}M`;
}

/**
 * Whether the phase represents active work.
 */
function isActivePhase(phase: AgentPhase): boolean {
  return (
    phase === 'thinking' ||
    phase === 'reading_file' ||
    phase === 'editing_file' ||
    phase === 'running_bash' ||
    phase === 'searching'
  );
}

/**
 * Build keybinding hints string matching Ratatui.
 * Clean, minimal hints — only the essentials.
 */
function getHints(_mode: AppMode, phase: AgentPhase): string {
  if (isActivePhase(phase)) {
    return 'Esc:scroll \u00B7 Ctrl+C:stop \u00B7 Ctrl+B:bg \u00B7 Ctrl+P:mode \u00B7 F2:sidebar';
  }
  return 'Esc:scroll \u00B7 Ctrl+C:quit \u00B7 Ctrl+B:bg \u00B7 Ctrl+P:mode \u00B7 F2:sidebar';
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function StatusBar(): React.ReactElement {
  const {
    phase,
    phaseLabel,
    elapsedMs,
    tokensRemaining,
    mode,
    voiceRecording,
  } = useUIState();
  const { colors } = useTheme();

  // Track whether the "done" checkmark is still visible (auto-hides after 2s).
  const [showDone, setShowDone] = useState(false);
  const doneTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (phase === 'done') {
      setShowDone(true);
      doneTimerRef.current = setTimeout(() => {
        setShowDone(false);
      }, 2000);
    } else if (phase !== 'idle') {
      setShowDone(false);
    }

    return () => {
      if (doneTimerRef.current) {
        clearTimeout(doneTimerRef.current);
      }
    };
  }, [phase]);

  const active = isActivePhase(phase);
  const hints = getHints(mode, phase);

  // -- Left side --
  let leftContent: React.ReactElement | null = null;

  if (voiceRecording) {
    leftContent = (
      <Text color={colors.status.error} bold>
        {'  \u{1F3A4} Recording...  [Space/Enter to stop \u00B7 Esc to cancel]'}
      </Text>
    );
  } else if (active) {
    const elapsed = formatElapsed(elapsedMs);
    const tokenPart = tokensRemaining != null
      ? ` \u00B7 \u2193 ${formatTokens(tokensRemaining)} tokens`
      : '';
    const statsStr = `(${elapsed}${tokenPart})`;

    leftContent = (
      <Box>
        <OaSpinner active label={phaseLabel || 'Working...'} />
        <Text color={colors.text.secondary}> {statsStr}</Text>
      </Box>
    );
  } else if (phase === 'done' && showDone) {
    leftContent = (
      <Text color={colors.status.done} bold>
        {'\u2713'} Done
      </Text>
    );
  } else if (phase === 'error') {
    leftContent = (
      <Text color={colors.status.error} bold>
        {'\u2717'} Error
      </Text>
    );
  }

  // -- Right side: all hints --
  const rightContent = (
    <Text color={colors.text.secondary}>{hints}</Text>
  );

  return (
    <Box width="100%" justifyContent="space-between">
      <Box flexShrink={1}>
        {leftContent}
      </Box>
      <Box flexShrink={0}>
        {rightContent}
      </Box>
    </Box>
  );
}
