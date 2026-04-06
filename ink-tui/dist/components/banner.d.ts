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
import React from 'react';
export interface BannerProps {
    /** Application version string (e.g. "2.0.10"). */
    version: string;
    /** User display name. */
    username: string;
    /** User email (optional). */
    email?: string;
    /** Organization name (optional). */
    org?: string;
    /** Current working directory path. */
    workingDir: string;
    /** Provider display name (e.g. "Anthropic"). */
    provider?: string;
    /** Model display name (e.g. "claude-sonnet-4-20250514"). */
    modelDisplay?: string;
    /** Credit balance (optional). */
    credits?: string;
    /** Tips list for the right column. */
    tips: string[];
    /** Available terminal width for sizing. Default: 80. */
    terminalWidth?: number;
}
export declare function Banner({ version, username, email, org, workingDir, provider, modelDisplay, credits, tips, terminalWidth, }: BannerProps): React.ReactElement;
