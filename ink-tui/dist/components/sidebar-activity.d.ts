/**
 * SidebarActivity -- Activity summary section for the sidebar panel.
 *
 * Matches Ratatui design exactly:
 *   updown-arrow N tool calls   (cyan icon)
 *   down-arrow N tokens         (green icon)
 *   clock Ns elapsed            (yellow icon)
 *   F mode: full-access         (red F for full-access, etc.)
 *
 * This section has no navigable items -- it is display-only.
 */
import React from 'react';
import type { ActivityInfo } from '../types/messages.js';
import type { SemanticColors } from '../themes/semantic-tokens.js';
export interface SidebarActivityProps {
    activity: ActivityInfo;
    isFocused: boolean;
    colors: SemanticColors;
    /** Current permission mode for display. */
    permissionMode?: string;
    /** Elapsed seconds since session start. */
    elapsedSecs?: number;
    /** Total tokens used. */
    totalTokens?: number;
}
export declare function SidebarActivity({ activity, isFocused, colors, permissionMode, elapsedSecs, totalTokens, }: SidebarActivityProps): React.ReactElement;
