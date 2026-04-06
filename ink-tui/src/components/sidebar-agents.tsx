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
import { Box, Text } from 'ink';
import type { AgentInfo, AgentStatus } from '../types/messages.js';
import type { SemanticColors } from '../themes/semantic-tokens.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Max display width for agent task summary (sidebar is 26ch, minus borders/icons). */
const MAX_TEXT_WIDTH = 20;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function statusIcon(status: AgentStatus): string {
  switch (status) {
    case 'Pending':   return '◦';
    case 'Running':   return '●';
    case 'Completed': return '✓';
    case 'Failed':    return '✗';
  }
}

function statusColor(status: AgentStatus, colors: SemanticColors): string {
  switch (status) {
    case 'Pending':   return colors.text.secondary;
    case 'Running':   return colors.status.running;
    case 'Completed': return colors.status.done;
    case 'Failed':    return colors.status.error;
  }
}

function truncate(text: string, maxLen: number): string {
  if (text.length <= maxLen) return text;
  return text.slice(0, maxLen - 1) + '…';
}

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface SidebarAgentsProps {
  agents: readonly AgentInfo[];
  selectedIndex: number;
  isFocused: boolean;
  colors: SemanticColors;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function SidebarAgents({
  agents,
  selectedIndex,
  isFocused,
  colors,
}: SidebarAgentsProps): React.ReactElement {
  if (agents.length === 0) {
    return (
      <Text color={colors.text.secondary}>  (none active)</Text>
    );
  }

  return (
    <Box flexDirection="column">
      {agents.map((agent, i) => {
        const isSelected = isFocused && i === selectedIndex;
        const icon = statusIcon(agent.status);
        const iconColor = statusColor(agent.status, colors);
        const label = truncate(agent.taskSummary || agent.agentId, MAX_TEXT_WIDTH);

        return (
          <Box key={agent.agentId}>
            <Text color={iconColor}> {icon} </Text>
            <Text
              color={isSelected ? colors.sidebar.itemSelected : colors.sidebar.itemDefault}
              bold={isSelected}
            >
              {label}
            </Text>
          </Box>
        );
      })}
    </Box>
  );
}
