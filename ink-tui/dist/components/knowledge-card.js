import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
const MAX_RESULTS_VISIBLE = 8;
const MAX_TAB_LABEL_LEN = 20;
const MAX_SNIPPET_LEN = 60;
const MAX_ANSWER_LINES = 30;
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
function truncate(s, maxLen) {
    if (s.length <= maxLen)
        return s;
    if (maxLen > 3)
        return s.slice(0, maxLen - 3) + '...';
    return s.slice(0, maxLen);
}
function formatDuration(ms) {
    if (ms < 1000)
        return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
}
// ---------------------------------------------------------------------------
// Title bar (rendered outside the Box since Ink doesn't support title on Box)
// We render the title as the first child inside the bordered box.
// ---------------------------------------------------------------------------
function TitleBar({ intent, durationMs, cached, expanded, borderColor, cacheColor, }) {
    const chevron = expanded ? '▾' : '▸';
    return (_jsxs(Box, { children: [_jsx(Text, { color: borderColor, children: '─ ' }), _jsx(Text, { color: borderColor, bold: true, children: '✦ ' }), _jsx(Text, { color: borderColor, bold: true, children: "KB" }), _jsx(Text, { color: borderColor, children: ' ── ' }), _jsx(Text, { color: borderColor, children: intent }), _jsx(Text, { color: borderColor, children: ` ── ${formatDuration(durationMs)} ` }), cached && (_jsx(Text, { color: cacheColor, children: '── ⚡cached ' })), _jsx(Text, { color: borderColor, children: chevron })] }));
}
// ---------------------------------------------------------------------------
// Tab bar
// ---------------------------------------------------------------------------
function TabBar({ tabs, activeIndex, activeColor, inactiveColor, }) {
    return (_jsx(Box, { children: tabs.map((tab, i) => {
            const label = truncate(tab.question, MAX_TAB_LABEL_LEN);
            if (i === activeIndex) {
                return (_jsx(Text, { backgroundColor: activeColor, color: "#000000", bold: true, children: ` [${label}] ` }, i));
            }
            return (_jsx(Text, { color: inactiveColor, children: `  ${label}  ` }, i));
        }) }));
}
// ---------------------------------------------------------------------------
// Result row
// ---------------------------------------------------------------------------
function ResultRow({ result, scoreColor, citationColor, graphColor, textColor, }) {
    const scorePct = Math.round(result.score * 100);
    const graphTag = result.graphExpanded ? ' ⇔' : '';
    return (_jsxs(Box, { flexDirection: "column", children: [_jsxs(Box, { children: [_jsx(Text, { color: citationColor, bold: true, children: result.citationLabel }), result.graphExpanded && (_jsx(Text, { color: graphColor, children: graphTag })), _jsxs(Text, { color: scoreColor, children: [" (", scorePct, "%) "] }), _jsx(Text, { color: textColor, dimColor: true, children: result.categoryLabel })] }), _jsx(Text, { color: textColor, children: truncate(result.snippet, MAX_SNIPPET_LEN) })] }));
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function KnowledgeCard({ queryId, subQuestions, answer, cached, graphExpanded, durationMs, expanded, activeTabIndex, onToggleExpand, onTabChange, isFocused, }) {
    const { colors } = useTheme();
    // Determine border color based on expanded state.
    const borderColor = expanded
        ? colors.knowledgeCard.expandedBorder
        : colors.knowledgeCard.border;
    // Focused override.
    const effectiveBorderColor = isFocused ? colors.border.focus : borderColor;
    // Derive intent from first sub-question if available.
    // (The Rust widget has an explicit intent field on the card; here we pass
    //  it through the title. The engine event has `intent` at top level.)
    const intent = subQuestions.length > 0 ? 'Strategic' : 'Direct';
    // Total results across all sub-questions.
    const totalResults = subQuestions.reduce((acc, sq) => acc + sq.results.length, 0);
    // Clamp active tab index.
    const safeTabIndex = Math.min(activeTabIndex, subQuestions.length - 1);
    const activeTab = subQuestions[safeTabIndex >= 0 ? safeTabIndex : 0];
    return (_jsxs(Box, { flexDirection: "column", borderStyle: "round", borderColor: effectiveBorderColor, paddingX: 1, children: [_jsx(TitleBar, { intent: intent, durationMs: durationMs, cached: cached, expanded: expanded, borderColor: effectiveBorderColor, cacheColor: colors.knowledgeCard.cache }), !expanded && (_jsxs(Box, { children: [_jsxs(Text, { color: colors.text.primary, children: [totalResults, " results"] }), _jsxs(Text, { color: colors.text.secondary, children: [' ', '·', " ", subQuestions.length, " sub-queries"] })] })), expanded && (_jsxs(Box, { flexDirection: "column", marginTop: 0, children: [subQuestions.length > 1 && (_jsxs(Box, { flexDirection: "column", children: [_jsx(TabBar, { tabs: subQuestions, activeIndex: safeTabIndex, activeColor: colors.knowledgeCard.tabActive, inactiveColor: colors.knowledgeCard.tabInactive }), _jsx(Text, { children: ' ' })] })), activeTab != null && activeTab.results.slice(0, MAX_RESULTS_VISIBLE).map((result, i) => (_jsx(Box, { flexDirection: "column", marginBottom: 1, children: _jsx(ResultRow, { result: result, scoreColor: colors.knowledgeCard.score, citationColor: colors.knowledgeCard.citation, graphColor: colors.knowledgeCard.graph, textColor: colors.text.primary }) }, result.chunkId || i))), answer !== '' && (_jsxs(Box, { flexDirection: "column", marginTop: 1, children: [_jsx(Text, { color: colors.knowledgeCard.answerDivider, bold: true, children: "\u2500\u2500 Answer \u2500\u2500" }), answer.split('\n').slice(0, MAX_ANSWER_LINES).map((line, i) => (_jsx(Text, { color: colors.text.primary, children: line }, i)))] }))] }))] }));
}
//# sourceMappingURL=knowledge-card.js.map