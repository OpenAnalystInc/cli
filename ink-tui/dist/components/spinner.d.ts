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
import React from 'react';
/** Full cycle duration = GRADIENT_STEPS * TICK_MS = 6.4s (wraps smoothly). */
export interface OaSpinnerProps {
    /** Optional label rendered after the spinner character. */
    label?: string;
    /** Whether the spinner is active. When false, renders nothing. */
    active: boolean;
}
export declare function OaSpinner({ label, active }: OaSpinnerProps): React.ReactElement | null;
