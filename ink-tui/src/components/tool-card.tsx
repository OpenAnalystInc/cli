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

import React, { useState, useEffect, useRef, useCallback } from 'react';
import { Box, Text } from 'ink';
import type { DiffInfo } from '../types/messages.js';
import { useTheme } from '../contexts/theme-context.js';
import { DiffView } from './diff-view.js';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface ToolCardProps {
  /** Unique tool call ID. */
  toolId: string;
  /** Tool name — e.g. "Bash", "Read", "Edit", "Write". */
  toolName: string;
  /** Execution status. */
  status: 'running' | 'completed' | 'failed';
  /** Tool input preview string. */
  input: string;
  /** Tool output (populated after completion). */
  output?: string;
  /** Execution duration in milliseconds. */
  durationMs?: number;
  /** Structured diff data for Edit/Write tools. */
  diff?: DiffInfo;
  /** Whether the output section is expanded. */
  expanded: boolean;
  /** Callback to toggle expand/collapse. */
  onToggleExpand: () => void;
  /** Whether this card is focused in scroll mode. */
  isFocused: boolean;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Braille spinner frames — same as Rust spinner. */
const SPINNER_FRAMES = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'] as const;
const SPINNER_INTERVAL_MS = 100;
const MAX_INPUT_CHARS = 60;
const MAX_OUTPUT_LINES = 20;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function truncate(s: string, maxLen: number): string {
  if (s.length <= maxLen) return s;
  if (maxLen > 3) return s.slice(0, maxLen - 3) + '...';
  return s.slice(0, maxLen);
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

/** Extract first line and truncate it. */
function inputPreview(input: string): string {
  const firstLine = input.split('\n')[0] ?? '';
  return truncate(firstLine, MAX_INPUT_CHARS);
}

// ---------------------------------------------------------------------------
// Spinner hook — animates through brand gradient frames
// ---------------------------------------------------------------------------

function useSpinner(active: boolean): { frame: string; color: string } {
  const [frameIndex, setFrameIndex] = useState(0);
  const { getSpinnerGradient } = useTheme();
  const gradientRef = useRef<readonly string[]>(getSpinnerGradient(SPINNER_FRAMES.length));

  useEffect(() => {
    if (!active) return;
    const interval = setInterval(() => {
      setFrameIndex((prev) => (prev + 1) % SPINNER_FRAMES.length);
    }, SPINNER_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [active]);

  return {
    frame: SPINNER_FRAMES[frameIndex] ?? '⠋',
    color: gradientRef.current[frameIndex] ?? '#3282FF',
  };
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function ToolCard({
  toolId,
  toolName,
  status,
  input,
  output,
  durationMs,
  diff,
  expanded,
  onToggleExpand,
  isFocused,
}: ToolCardProps): React.ReactElement {
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
      ? { char: '✓', color: colors.status.done }
      : { char: '✗', color: colors.status.error };

  // Duration label.
  const durationLabel = durationMs != null ? formatDuration(durationMs) : '';

  // Expand chevron.
  const chevron = expanded ? '▾' : '▸';

  // Output lines for expanded view.
  const outputLines = output?.split('\n') ?? [];
  const visibleOutputLines = outputLines.slice(0, MAX_OUTPUT_LINES);
  const overflowCount = outputLines.length - MAX_OUTPUT_LINES;

  return (
    <Box
      flexDirection="column"
      borderStyle="round"
      borderColor={effectiveBorderColor}
      paddingX={1}
    >
      {/* Title line: icon + tool name + duration + chevron */}
      <Box>
        <Text color={statusIcon.color}>{statusIcon.char} </Text>
        <Text color={colors.text.accent} bold>{toolName}</Text>
        {durationLabel !== '' && (
          <Text color={colors.text.secondary}> {' '}── {durationLabel} </Text>
        )}
        <Text color={effectiveBorderColor}> {chevron}</Text>
      </Box>

      {/* Input preview: first line of tool input */}
      <Text color={colors.text.primary}>
        {inputPreview(input)}
      </Text>

      {/* Expanded content */}
      {expanded && (
        <Box flexDirection="column" marginTop={1}>
          {/* If we have a structured diff, render DiffView */}
          {diff != null ? (
            <DiffView
              filePath={diff.filePath}
              added={diff.added}
              removed={diff.removed}
              hunks={diff.hunks}
              maxLines={MAX_OUTPUT_LINES}
            />
          ) : output != null ? (
            <Box flexDirection="column">
              {visibleOutputLines.map((line, i) => (
                <Text key={i} color={colors.text.primary}>
                  {line}
                </Text>
              ))}
              {overflowCount > 0 && (
                <Text color={colors.text.secondary} dimColor>
                  ... ({overflowCount} more lines)
                </Text>
              )}
            </Box>
          ) : null}
        </Box>
      )}
    </Box>
  );
}
