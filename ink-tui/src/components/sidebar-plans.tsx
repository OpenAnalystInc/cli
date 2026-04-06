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
import { Box, Text } from 'ink';
import type { PlanInfo } from '../types/messages.js';
import type { SemanticColors } from '../themes/semantic-tokens.js';

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const MAX_NAME_WIDTH = 14;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function statusLabel(status: PlanInfo['status']): string {
  switch (status) {
    case 'todo':        return '[TODO]';
    case 'in_progress': return '[WIP]';
    case 'done':        return '[DONE]';
  }
}

function statusColor(status: PlanInfo['status'], colors: SemanticColors): string {
  switch (status) {
    case 'todo':        return colors.text.secondary;
    case 'in_progress': return colors.status.warning;
    case 'done':        return colors.status.done;
  }
}

function truncate(text: string, maxLen: number): string {
  if (text.length <= maxLen) return text;
  return text.slice(0, maxLen - 1) + '…';
}

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface SidebarPlansProps {
  plans: readonly PlanInfo[];
  selectedIndex: number;
  isFocused: boolean;
  colors: SemanticColors;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function SidebarPlans({
  plans,
  selectedIndex,
  isFocused,
  colors,
}: SidebarPlansProps): React.ReactElement {
  if (plans.length === 0) {
    return (
      <Text color={colors.text.secondary}>  (no plans)</Text>
    );
  }

  return (
    <Box flexDirection="column">
      {plans.map((plan, i) => {
        const isSelected = isFocused && i === selectedIndex;
        const label = statusLabel(plan.status);
        const labelColor = statusColor(plan.status, colors);
        const name = truncate(plan.name, MAX_NAME_WIDTH);

        return (
          <Box key={plan.name}>
            <Text color={labelColor}> {label} </Text>
            <Text
              color={isSelected ? colors.sidebar.itemSelected : colors.sidebar.itemDefault}
              bold={isSelected}
            >
              {name}
            </Text>
          </Box>
        );
      })}
    </Box>
  );
}
