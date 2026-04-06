/**
 * SidebarPlans — Plans section for the sidebar panel.
 *
 * Displays plans from `.openanalyst/plans/` with status labels:
 *   - `[TODO]`        dim
 *   - `[IN PROGRESS]` yellow/warning
 *   - `[DONE]`        green/done
 *
 * Enter on a selected plan emits an action to open it in the editor.
 */
import React from 'react';
import type { PlanInfo } from '../types/messages.js';
import type { SemanticColors } from '../themes/semantic-tokens.js';
export interface SidebarPlansProps {
    plans: readonly PlanInfo[];
    selectedIndex: number;
    isFocused: boolean;
    colors: SemanticColors;
}
export declare function SidebarPlans({ plans, selectedIndex, isFocused, colors, }: SidebarPlansProps): React.ReactElement;
