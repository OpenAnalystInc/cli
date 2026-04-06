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
import React from 'react';
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
export declare function Banner({ version, username, email, org, workingDir, provider, modelDisplay, credits, tips, terminalWidth, }: BannerProps): React.ReactElement;
