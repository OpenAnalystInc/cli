/**
 * OaSpinner — animated braille spinner with brand-color gradient cycling.
 *
 * Mirrors the Rust `crates/tui-widgets/src/spinner.rs` behavior:
 *   - Braille cycle: 10 frames at 100ms tick (~10fps)
 *   - Color: interpolated through the OA brand gradient over ~4 seconds
 *   - Uses `useTheme().getSpinnerGradient(64)` for a smooth 64-step palette
 *
 * Inspired by Gemini CLI's GeminiSpinner but adapted for Ink + OA brand.
 */

import React, { useState, useEffect, useMemo } from 'react';
import { Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Braille spinner frames (matches Rust throbber-widgets-tui "braille" set). */
const BRAILLE_FRAMES = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'] as const;

/** Tick interval in ms (~10fps). */
const TICK_MS = 100;

/** Number of gradient steps for one full color cycle. */
const GRADIENT_STEPS = 64;

/** Full cycle duration = GRADIENT_STEPS * TICK_MS = 6.4s (wraps smoothly). */

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface OaSpinnerProps {
  /** Optional label rendered after the spinner character. */
  label?: string;
  /** Whether the spinner is active. When false, renders nothing. */
  active: boolean;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function OaSpinner({ label, active }: OaSpinnerProps): React.ReactElement | null {
  const { getSpinnerGradient, colors } = useTheme();
  const [tick, setTick] = useState(0);

  // Pre-compute the full gradient palette once.
  const gradient = useMemo(() => getSpinnerGradient(GRADIENT_STEPS), [getSpinnerGradient]);

  useEffect(() => {
    if (!active) return;

    const id = setInterval(() => {
      setTick((prev) => prev + 1);
    }, TICK_MS);

    return () => clearInterval(id);
  }, [active]);

  if (!active) {
    return null;
  }

  const frameIndex = tick % BRAILLE_FRAMES.length;
  const colorIndex = tick % gradient.length;
  const frame = BRAILLE_FRAMES[frameIndex];
  const color = gradient[colorIndex] ?? colors.spinner.active;

  if (label) {
    return (
      <Text>
        <Text color={color}>{frame}</Text>
        <Text color={color}> {label}</Text>
      </Text>
    );
  }

  return <Text color={color}>{frame}</Text>;
}
