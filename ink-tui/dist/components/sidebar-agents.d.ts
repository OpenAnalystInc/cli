/**
 * SidebarAgents — Agents section for the sidebar panel.
 *
 * Displays running/available agents with status icons:
 *   - `◦` pending  (dimmed)
 *   - `●` running  (blue/running color)
 *   - `✓` completed (green/done color)
 *   - `✗` failed   (red/error color)
 *
 * The selected item is highlighted with sidebar.itemSelected.
 * Enter on a selected agent sets it as the active agent.
 */
import React from 'react';
import type { AgentInfo } from '../types/messages.js';
import type { SemanticColors } from '../themes/semantic-tokens.js';
export interface SidebarAgentsProps {
    agents: readonly AgentInfo[];
    selectedIndex: number;
    isFocused: boolean;
    colors: SemanticColors;
}
export declare function SidebarAgents({ agents, selectedIndex, isFocused, colors, }: SidebarAgentsProps): React.ReactElement;
