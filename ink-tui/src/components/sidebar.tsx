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

import React, { useState, useCallback, useMemo } from 'react';
import { Box, Text } from 'ink';
import { useUIState, useUIActions } from '../contexts/ui-state-context.js';
import { useTheme } from '../contexts/theme-context.js';

import { useKeypress } from '../hooks/use-keypress.js';
import { Command } from '../key/commands.js';
import type {
  AgentInfo,
  FileInfo,
  PlanInfo,
  RoutingTable,
  ActivityInfo,
  ActionCategory,
} from '../types/messages.js';

import { SidebarAgents } from './sidebar-agents.js';
import { SidebarFiles } from './sidebar-files.js';
import { SidebarPlans } from './sidebar-plans.js';
import { SidebarRouting } from './sidebar-routing.js';
import { SidebarActivity } from './sidebar-activity.js';

// ---------------------------------------------------------------------------
// Section enum
// ---------------------------------------------------------------------------

enum Section {
  Agents   = 0,
  Files    = 1,
  Plans    = 2,
  Routing  = 3,
  Activity = 4,
}

const SECTION_COUNT = 5;

const SECTION_TITLES: Record<Section, string> = {
  [Section.Agents]:   'Agents',
  [Section.Files]:    'Files',
  [Section.Plans]:    'Plans',
  [Section.Routing]:  'Routing',
  [Section.Activity]: 'Activity',
};

/**
 * Section header colors resolved at render time from semantic tokens.
 * Agents=error (red/orange), Files/Plans/Routing=accent (cyan), Activity=done (green).
 */

// ---------------------------------------------------------------------------
// Default empty data
// ---------------------------------------------------------------------------

const EMPTY_AGENTS: readonly AgentInfo[] = [];
const EMPTY_FILES: readonly FileInfo[] = [];
const EMPTY_PLANS: readonly PlanInfo[] = [];

const EMPTY_ROUTING: RoutingTable = {
  explore:  { model: '', tier: '' },
  research: { model: '', tier: '' },
  code:     { model: '', tier: '' },
  write:    { model: '', tier: '' },
};

const EMPTY_ACTIVITY: ActivityInfo = {
  backgroundTasks: 0,
  toolCallCount: 0,
  mcpServers: 0,
};

const TIER_CYCLE = ['fast', 'balanced', 'capable'] as const;

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Number of navigable items in a given section. */
function itemCount(
  section: Section,
  agents: readonly AgentInfo[],
  files: readonly FileInfo[],
  plans: readonly PlanInfo[],
): number {
  switch (section) {
    case Section.Agents:   return agents.length;
    case Section.Files:    return files.length;
    case Section.Plans:    return plans.length;
    case Section.Routing:  return 4; // always 4 categories
    case Section.Activity: return 0; // display-only, not navigable
  }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function Sidebar({
  agents = EMPTY_AGENTS,
  files = EMPTY_FILES,
  plans = EMPTY_PLANS,
  routing = EMPTY_ROUTING,
  activity = EMPTY_ACTIVITY,
  onRoutingChange,
  onAgentSelect,
  onFileToggle,
  onPlanSelect,
}: SidebarProps): React.ReactElement {
  const { sidebarFocused, permissionMode, elapsedMs, tokensRemaining, totalInputTokens, totalOutputTokens } = useUIState();
  const actions = useUIActions();
  const { colors } = useTheme();

  // -- Local state --
  const [activeSection, setActiveSection] = useState<Section>(Section.Agents);
  const [selectedIndices, setSelectedIndices] = useState<Record<Section, number>>({
    [Section.Agents]: 0,
    [Section.Files]: 0,
    [Section.Plans]: 0,
    [Section.Routing]: 0,
    [Section.Activity]: 0,
  });
  const [collapsed, setCollapsed] = useState<Record<Section, boolean>>({
    [Section.Agents]: false,
    [Section.Files]: false,
    [Section.Plans]: false,
    [Section.Routing]: false,
    [Section.Activity]: false,
  });

  // Memoize routing categories for tier cycling.
  const routingCategories = useMemo<readonly ActionCategory[]>(
    () => ['explore', 'research', 'code', 'write'],
    [],
  );

  // -- Keypress handler --
  const handleKeypress = useCallback(
    (_input: string, _key: unknown, command: Command | undefined): boolean => {
      if (!command) return false;

      switch (command) {
        // --- Section navigation ---
        // Tab/Shift+Tab may resolve to NEXT_TAB/PREV_TAB (earlier in enum)
        // instead of the sidebar-specific commands. Accept both.
        case Command.SIDEBAR_NEXT_SECTION:
        case Command.NEXT_TAB: {
          setActiveSection((prev) => ((prev + 1) % SECTION_COUNT) as Section);
          return true;
        }

        case Command.SIDEBAR_PREV_SECTION:
        case Command.PREV_TAB: {
          setActiveSection((prev) => ((prev - 1 + SECTION_COUNT) % SECTION_COUNT) as Section);
          return true;
        }

        // --- Item navigation ---
        // j/Down resolve to SCROLL_DOWN or HISTORY_DOWN (earlier in enum)
        // rather than SIDEBAR_NEXT_ITEM. Accept all equivalent commands.
        case Command.SIDEBAR_NEXT_ITEM:
        case Command.SCROLL_DOWN:
        case Command.HISTORY_DOWN: {
          const max = itemCount(activeSection, agents, files, plans);
          if (max === 0) return true;
          setSelectedIndices((prev) => ({
            ...prev,
            [activeSection]: Math.min(prev[activeSection] + 1, max - 1),
          }));
          return true;
        }

        // k/Up resolve to SCROLL_UP or HISTORY_UP (earlier in enum)
        // rather than SIDEBAR_PREV_ITEM. Accept all equivalent commands.
        case Command.SIDEBAR_PREV_ITEM:
        case Command.SCROLL_UP:
        case Command.HISTORY_UP: {
          setSelectedIndices((prev) => ({
            ...prev,
            [activeSection]: Math.max(prev[activeSection] - 1, 0),
          }));
          return true;
        }

        // --- Action on selected item ---
        // Return resolves to SUBMIT (first in enum) rather than
        // SIDEBAR_ACTION. Accept both when sidebar is focused.
        case Command.SIDEBAR_ACTION:
        case Command.SUBMIT: {
          // If the section is collapsed, expand it instead of performing action
          if (collapsed[activeSection]) {
            setCollapsed((prev) => ({ ...prev, [activeSection]: false }));
            return true;
          }

          const idx = selectedIndices[activeSection];

          switch (activeSection) {
            case Section.Agents: {
              const agent = agents[idx];
              if (agent) onAgentSelect?.(agent.agentId);
              break;
            }
            case Section.Files: {
              const file = files[idx];
              if (file) onFileToggle?.(file.path);
              break;
            }
            case Section.Plans: {
              const plan = plans[idx];
              if (plan) onPlanSelect?.(plan.name);
              break;
            }
            case Section.Routing: {
              const cat = routingCategories[idx];
              if (cat) {
                const currentTier = routing[cat].tier;
                const currentIdx = TIER_CYCLE.indexOf(currentTier as typeof TIER_CYCLE[number]);
                const nextTier = TIER_CYCLE[(currentIdx + 1) % TIER_CYCLE.length];
                onRoutingChange?.(cat, nextTier);
              }
              break;
            }
            case Section.Activity:
              // Toggle collapse since Activity has no actionable items
              setCollapsed((prev) => ({ ...prev, [activeSection]: !prev[activeSection] }));
              break;
          }
          return true;
        }

        // --- Exit sidebar focus (keep visible, return focus to input) ---
        case Command.SIDEBAR_EXIT:
        case Command.EXIT_SCROLL_MODE:
        case Command.ENTER_SCROLL_MODE: {
          // The command resolver iterates enum values in order, so shared
          // key bindings (Escape, i) may resolve to whichever command
          // appears first: ENTER_SCROLL_MODE or EXIT_SCROLL_MODE instead
          // of SIDEBAR_EXIT. Accept all three when sidebar is focused.
          actions.toggleSidebar(); // Focused -> unfocus (stays visible)
          return true;
        }

        default:
          return false;
      }
    },
    [
      activeSection, selectedIndices, collapsed, agents, files, plans,
      routing, routingCategories, onRoutingChange, onAgentSelect,
      onFileToggle, onPlanSelect, actions,
    ],
  );

  useKeypress(handleKeypress, {
    isActive: sidebarFocused,
    priority: 5,
  });

  // -- Resolve section header color from semantic tokens --
  const getSectionColor = (section: Section): string => {
    switch (section) {
      case Section.Agents:   return colors.status.error;   // red/orange
      case Section.Files:    return colors.text.accent;    // cyan
      case Section.Plans:    return colors.text.accent;    // cyan
      case Section.Routing:  return colors.text.accent;    // cyan
      case Section.Activity: return colors.status.done;    // green
    }
  };

  // -- Section header renderer (matches Ratatui design) --
  const renderSectionHeader = (section: Section): React.ReactElement => {
    const isActive = sidebarFocused && section === activeSection;
    const title = SECTION_TITLES[section];

    if (isActive) {
      // Focused: yellow bold with arrows (matches Ratatui: `>> Title <<` style)
      return (
        <Box key={`header-${section}`}>
          <Text color={colors.status.warning} bold backgroundColor={colors.sidebar.border}>
            {'\u25B8'} {title} {'\u25C2'}
          </Text>
        </Box>
      );
    }

    // Unfocused: section-specific color, bold
    const headerColor = getSectionColor(section);
    return (
      <Box key={`header-${section}`}>
        <Text color={headerColor} bold>
          {'  '}{title}
        </Text>
      </Box>
    );
  };

  // -- Section separator --
  const renderSeparator = (): React.ReactElement => {
    return <Text color={colors.sidebar.border}>{' \u2500'.repeat(11)}</Text>;
  };

  // -- Section content renderer --
  const renderSectionContent = (section: Section): React.ReactElement | null => {
    if (collapsed[section]) return null;

    const isFocused = sidebarFocused && section === activeSection;
    const idx = selectedIndices[section];

    switch (section) {
      case Section.Agents:
        return (
          <SidebarAgents
            agents={agents}
            selectedIndex={idx}
            isFocused={isFocused}
            colors={colors}
          />
        );
      case Section.Files:
        return (
          <SidebarFiles
            files={files}
            selectedIndex={idx}
            isFocused={isFocused}
            colors={colors}
          />
        );
      case Section.Plans:
        return (
          <SidebarPlans
            plans={plans}
            selectedIndex={idx}
            isFocused={isFocused}
            colors={colors}
          />
        );
      case Section.Routing:
        return (
          <SidebarRouting
            routing={routing}
            selectedIndex={idx}
            isFocused={isFocused}
            colors={colors}
          />
        );
      case Section.Activity:
        return (
          <SidebarActivity
            activity={activity}
            isFocused={isFocused}
            colors={colors}
            permissionMode={permissionMode}
            elapsedSecs={Math.floor(elapsedMs / 1000)}
            totalTokens={totalInputTokens + totalOutputTokens}
          />
        );
    }
  };

  // -- Render all sections --
  const sections = [
    Section.Agents,
    Section.Files,
    Section.Plans,
    Section.Routing,
    Section.Activity,
  ];

  return (
    <Box flexDirection="column" paddingX={0}>
      {/* Sections */}
      {sections.map((section, i) => (
        <Box key={section} flexDirection="column">
          {renderSectionHeader(section)}
          {renderSectionContent(section)}
          {i < sections.length - 1 && renderSeparator()}
        </Box>
      ))}

      {/* Footer hint */}
      <Box marginTop={1}>
        <Text color={colors.text.secondary} dimColor>
          {sidebarFocused
            ? 'Tab:section j/k:nav Esc:'
            : 'Tab:section j/k:nav Esc:'}
        </Text>
      </Box>
    </Box>
  );
}
