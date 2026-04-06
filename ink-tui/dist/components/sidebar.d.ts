/**
 * Sidebar -- 5-section collapsible sidebar panel (26 chars fixed width).
 *
 * Sections (matching Ratatui design):
 *   1. Agents   -- running/available agents with status icons
 *   2. Files    -- touched files with action icons
 *   3. Plans    -- plans with status labels
 *   4. Routing  -- model routing table (4 categories with colored dots)
 *   5. Activity -- tool calls, tokens, elapsed, permission mode
 *
 * Keyboard navigation (when sidebar is focused, priority 5):
 *   - Tab/Shift+Tab: cycle between sections
 *   - j/k (or Down/Up): navigate items within active section
 *   - Enter: perform section-specific action
 *   - Esc/i: return focus to input
 *
 * Section headers match Ratatui:
 *   - Focused:   `>> Title <<` in yellow bold with bg highlight
 *   - Unfocused: `  Title` in section-specific color bold
 */
import React from 'react';
import type { AgentInfo, FileInfo, PlanInfo, RoutingTable, ActivityInfo, ActionCategory } from '../types/messages.js';
export interface SidebarProps {
    agents?: readonly AgentInfo[];
    files?: readonly FileInfo[];
    plans?: readonly PlanInfo[];
    routing?: RoutingTable;
    activity?: ActivityInfo;
    /** Called when user cycles a routing tier. */
    onRoutingChange?: (category: ActionCategory, tier: string) => void;
    /** Called when user selects an agent. */
    onAgentSelect?: (agentId: string) => void;
    /** Called when user toggles a context file. */
    onFileToggle?: (path: string) => void;
    /** Called when user selects a plan. */
    onPlanSelect?: (name: string) => void;
}
export declare function Sidebar({ agents, files, plans, routing, activity, onRoutingChange, onAgentSelect, onFileToggle, onPlanSelect, }: SidebarProps): React.ReactElement;
