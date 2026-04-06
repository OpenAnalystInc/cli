import { jsxs as _jsxs, jsx as _jsx } from "react/jsx-runtime";
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
import { useState, useCallback, useMemo } from 'react';
import { Box, Text } from 'ink';
import { useUIState, useUIActions } from '../contexts/ui-state-context.js';
import { useTheme } from '../contexts/theme-context.js';
import { useKeypress } from '../hooks/use-keypress.js';
import { Command } from '../key/commands.js';
import { SidebarAgents } from './sidebar-agents.js';
import { SidebarFiles } from './sidebar-files.js';
import { SidebarPlans } from './sidebar-plans.js';
import { SidebarRouting } from './sidebar-routing.js';
import { SidebarActivity } from './sidebar-activity.js';
// ---------------------------------------------------------------------------
// Section enum
// ---------------------------------------------------------------------------
var Section;
(function (Section) {
    Section[Section["Agents"] = 0] = "Agents";
    Section[Section["Files"] = 1] = "Files";
    Section[Section["Plans"] = 2] = "Plans";
    Section[Section["Routing"] = 3] = "Routing";
    Section[Section["Activity"] = 4] = "Activity";
})(Section || (Section = {}));
const SECTION_COUNT = 5;
const SECTION_TITLES = {
    [Section.Agents]: 'Agents',
    [Section.Files]: 'Files',
    [Section.Plans]: 'Plans',
    [Section.Routing]: 'Routing',
    [Section.Activity]: 'Activity',
};
/**
 * Section header colors resolved at render time from semantic tokens.
 * Agents=error (red/orange), Files/Plans/Routing=accent (cyan), Activity=done (green).
 */
// ---------------------------------------------------------------------------
// Default empty data
// ---------------------------------------------------------------------------
const EMPTY_AGENTS = [];
const EMPTY_FILES = [];
const EMPTY_PLANS = [];
const EMPTY_ROUTING = {
    explore: { model: '', tier: '' },
    research: { model: '', tier: '' },
    code: { model: '', tier: '' },
    write: { model: '', tier: '' },
};
const EMPTY_ACTIVITY = {
    backgroundTasks: 0,
    toolCallCount: 0,
    mcpServers: 0,
};
const TIER_CYCLE = ['fast', 'balanced', 'capable'];
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
/** Number of navigable items in a given section. */
function itemCount(section, agents, files, plans) {
    switch (section) {
        case Section.Agents: return agents.length;
        case Section.Files: return files.length;
        case Section.Plans: return plans.length;
        case Section.Routing: return 4; // always 4 categories
        case Section.Activity: return 0; // display-only, not navigable
    }
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function Sidebar({ agents = EMPTY_AGENTS, files = EMPTY_FILES, plans = EMPTY_PLANS, routing = EMPTY_ROUTING, activity = EMPTY_ACTIVITY, onRoutingChange, onAgentSelect, onFileToggle, onPlanSelect, }) {
    const { sidebarFocused, permissionMode, elapsedMs, tokensRemaining, totalInputTokens, totalOutputTokens } = useUIState();
    const actions = useUIActions();
    const { colors } = useTheme();
    // -- Local state --
    const [activeSection, setActiveSection] = useState(Section.Agents);
    const [selectedIndices, setSelectedIndices] = useState({
        [Section.Agents]: 0,
        [Section.Files]: 0,
        [Section.Plans]: 0,
        [Section.Routing]: 0,
        [Section.Activity]: 0,
    });
    const [collapsed, setCollapsed] = useState({
        [Section.Agents]: false,
        [Section.Files]: false,
        [Section.Plans]: false,
        [Section.Routing]: false,
        [Section.Activity]: false,
    });
    // Memoize routing categories for tier cycling.
    const routingCategories = useMemo(() => ['explore', 'research', 'code', 'write'], []);
    // -- Keypress handler --
    const handleKeypress = useCallback((_input, _key, command) => {
        if (!command)
            return false;
        switch (command) {
            // --- Section navigation ---
            // Tab/Shift+Tab may resolve to NEXT_TAB/PREV_TAB (earlier in enum)
            // instead of the sidebar-specific commands. Accept both.
            case Command.SIDEBAR_NEXT_SECTION:
            case Command.NEXT_TAB: {
                setActiveSection((prev) => ((prev + 1) % SECTION_COUNT));
                return true;
            }
            case Command.SIDEBAR_PREV_SECTION:
            case Command.PREV_TAB: {
                setActiveSection((prev) => ((prev - 1 + SECTION_COUNT) % SECTION_COUNT));
                return true;
            }
            // --- Item navigation ---
            // j/Down resolve to SCROLL_DOWN or HISTORY_DOWN (earlier in enum)
            // rather than SIDEBAR_NEXT_ITEM. Accept all equivalent commands.
            case Command.SIDEBAR_NEXT_ITEM:
            case Command.SCROLL_DOWN:
            case Command.HISTORY_DOWN: {
                const max = itemCount(activeSection, agents, files, plans);
                if (max === 0)
                    return true;
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
                        if (agent)
                            onAgentSelect?.(agent.agentId);
                        break;
                    }
                    case Section.Files: {
                        const file = files[idx];
                        if (file)
                            onFileToggle?.(file.path);
                        break;
                    }
                    case Section.Plans: {
                        const plan = plans[idx];
                        if (plan)
                            onPlanSelect?.(plan.name);
                        break;
                    }
                    case Section.Routing: {
                        const cat = routingCategories[idx];
                        if (cat) {
                            const currentTier = routing[cat].tier;
                            const currentIdx = TIER_CYCLE.indexOf(currentTier);
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
    }, [
        activeSection, selectedIndices, collapsed, agents, files, plans,
        routing, routingCategories, onRoutingChange, onAgentSelect,
        onFileToggle, onPlanSelect, actions,
    ]);
    useKeypress(handleKeypress, {
        isActive: sidebarFocused,
        priority: 5,
    });
    // -- Resolve section header color from semantic tokens --
    const getSectionColor = (section) => {
        switch (section) {
            case Section.Agents: return colors.status.error; // red/orange
            case Section.Files: return colors.text.accent; // cyan
            case Section.Plans: return colors.text.accent; // cyan
            case Section.Routing: return colors.text.accent; // cyan
            case Section.Activity: return colors.status.done; // green
        }
    };
    // -- Section header renderer (matches Ratatui design) --
    const renderSectionHeader = (section) => {
        const isActive = sidebarFocused && section === activeSection;
        const title = SECTION_TITLES[section];
        if (isActive) {
            // Focused: yellow bold with arrows (matches Ratatui: `>> Title <<` style)
            return (_jsx(Box, { children: _jsxs(Text, { color: colors.status.warning, bold: true, backgroundColor: colors.sidebar.border, children: ['\u25B8', " ", title, " ", '\u25C2'] }) }, `header-${section}`));
        }
        // Unfocused: section-specific color, bold
        const headerColor = getSectionColor(section);
        return (_jsx(Box, { children: _jsxs(Text, { color: headerColor, bold: true, children: ['  ', title] }) }, `header-${section}`));
    };
    // -- Section separator --
    const renderSeparator = () => {
        return _jsx(Text, { color: colors.sidebar.border, children: ' \u2500'.repeat(11) });
    };
    // -- Section content renderer --
    const renderSectionContent = (section) => {
        if (collapsed[section])
            return null;
        const isFocused = sidebarFocused && section === activeSection;
        const idx = selectedIndices[section];
        switch (section) {
            case Section.Agents:
                return (_jsx(SidebarAgents, { agents: agents, selectedIndex: idx, isFocused: isFocused, colors: colors }));
            case Section.Files:
                return (_jsx(SidebarFiles, { files: files, selectedIndex: idx, isFocused: isFocused, colors: colors }));
            case Section.Plans:
                return (_jsx(SidebarPlans, { plans: plans, selectedIndex: idx, isFocused: isFocused, colors: colors }));
            case Section.Routing:
                return (_jsx(SidebarRouting, { routing: routing, selectedIndex: idx, isFocused: isFocused, colors: colors }));
            case Section.Activity:
                return (_jsx(SidebarActivity, { activity: activity, isFocused: isFocused, colors: colors, permissionMode: permissionMode, elapsedSecs: Math.floor(elapsedMs / 1000), totalTokens: totalInputTokens + totalOutputTokens }));
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
    return (_jsxs(Box, { flexDirection: "column", paddingX: 0, children: [sections.map((section, i) => (_jsxs(Box, { flexDirection: "column", children: [renderSectionHeader(section), renderSectionContent(section), i < sections.length - 1 && renderSeparator()] }, section))), _jsx(Box, { marginTop: 1, children: _jsx(Text, { color: colors.text.secondary, dimColor: true, children: sidebarFocused
                        ? 'Tab:section j/k:nav Esc:'
                        : 'Tab:section j/k:nav Esc:' }) })] }));
}
//# sourceMappingURL=sidebar.js.map