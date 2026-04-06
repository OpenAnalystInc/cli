/**
 * KnowledgeCard — tabbed, collapsible inline card for KB query results.
 *
 * Mirrors the Rust tui-widgets/knowledge_card.rs widget:
 *
 * Collapsed:
 *   ╭─ ✦ KB ── Strategic ── 2.3s ── ⚡cached ▸ ╮
 *   │ 12 results · 3 sub-queries                │
 *   ╰───────────────────────────────────────────╯
 *
 * Expanded:
 *   ╭─ ✦ KB ── Strategic ── 2.3s ── ⚡cached ▾ ╮
 *   │  [Sub-Q A] Sub-Q B Sub-Q C                │
 *   │  [Result #1] ⇔ (95%) snippet...           │
 *   │  [Result #2]    (87%) snippet...           │
 *   │  ── Answer ──                              │
 *   │  Synthesized answer text...                │
 *   ╰───────────────────────────────────────────╯
 *
 * All colors from useTheme() semantic tokens.
 */

import React from 'react';
import { Box, Text } from 'ink';
import type { KbChunkResult } from '../types/messages.js';
import { useTheme } from '../contexts/theme-context.js';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface SubQuestion {
  question: string;
  results: KbChunkResult[];
}

export interface KnowledgeCardProps {
  /** Query ID for feedback association. */
  queryId: string;
  /** Sub-questions and their results. */
  subQuestions: SubQuestion[];
  /** Synthesized answer text. */
  answer: string;
  /** Whether this result was served from cache. */
  cached: boolean;
  /** Whether any result was found via graph expansion. */
  graphExpanded: boolean;
  /** Total query duration in milliseconds. */
  durationMs: number;
  /** Whether the card body is expanded. */
  expanded: boolean;
  /** Index of the active sub-question tab. */
  activeTabIndex: number;
  /** Toggle expand/collapse. */
  onToggleExpand: () => void;
  /** Switch active tab. */
  onTabChange: (index: number) => void;
  /** Whether this card is focused in scroll mode. */
  isFocused: boolean;
}

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

function truncate(s: string, maxLen: number): string {
  if (s.length <= maxLen) return s;
  if (maxLen > 3) return s.slice(0, maxLen - 3) + '...';
  return s.slice(0, maxLen);
}

function formatDuration(ms: number): string {
  if (ms < 1000) return `${ms}ms`;
  return `${(ms / 1000).toFixed(1)}s`;
}

// ---------------------------------------------------------------------------
// Title bar (rendered outside the Box since Ink doesn't support title on Box)
// We render the title as the first child inside the bordered box.
// ---------------------------------------------------------------------------

function TitleBar({
  intent,
  durationMs,
  cached,
  expanded,
  borderColor,
  cacheColor,
}: {
  intent: string;
  durationMs: number;
  cached: boolean;
  expanded: boolean;
  borderColor: string;
  cacheColor: string;
}): React.ReactElement {
  const chevron = expanded ? '▾' : '▸';
  return (
    <Box>
      <Text color={borderColor}>{'─ '}</Text>
      <Text color={borderColor} bold>{'✦ '}</Text>
      <Text color={borderColor} bold>KB</Text>
      <Text color={borderColor}>{' ── '}</Text>
      <Text color={borderColor}>{intent}</Text>
      <Text color={borderColor}>{` ── ${formatDuration(durationMs)} `}</Text>
      {cached && (
        <Text color={cacheColor}>{'── ⚡cached '}</Text>
      )}
      <Text color={borderColor}>{chevron}</Text>
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Tab bar
// ---------------------------------------------------------------------------

function TabBar({
  tabs,
  activeIndex,
  activeColor,
  inactiveColor,
}: {
  tabs: SubQuestion[];
  activeIndex: number;
  activeColor: string;
  inactiveColor: string;
}): React.ReactElement {
  return (
    <Box>
      {tabs.map((tab, i) => {
        const label = truncate(tab.question, MAX_TAB_LABEL_LEN);
        if (i === activeIndex) {
          return (
            <Text key={i} backgroundColor={activeColor} color="#000000" bold>
              {` [${label}] `}
            </Text>
          );
        }
        return (
          <Text key={i} color={inactiveColor}>
            {`  ${label}  `}
          </Text>
        );
      })}
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Result row
// ---------------------------------------------------------------------------

function ResultRow({
  result,
  scoreColor,
  citationColor,
  graphColor,
  textColor,
}: {
  result: KbChunkResult;
  scoreColor: string;
  citationColor: string;
  graphColor: string;
  textColor: string;
}): React.ReactElement {
  const scorePct = Math.round(result.score * 100);
  const graphTag = result.graphExpanded ? ' ⇔' : '';

  return (
    <Box flexDirection="column">
      <Box>
        <Text color={citationColor} bold>{result.citationLabel}</Text>
        {result.graphExpanded && (
          <Text color={graphColor}>{graphTag}</Text>
        )}
        <Text color={scoreColor}> ({scorePct}%) </Text>
        <Text color={textColor} dimColor>{result.categoryLabel}</Text>
      </Box>
      <Text color={textColor}>
        {truncate(result.snippet, MAX_SNIPPET_LEN)}
      </Text>
    </Box>
  );
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function KnowledgeCard({
  queryId,
  subQuestions,
  answer,
  cached,
  graphExpanded,
  durationMs,
  expanded,
  activeTabIndex,
  onToggleExpand,
  onTabChange,
  isFocused,
}: KnowledgeCardProps): React.ReactElement {
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
  const totalResults = subQuestions.reduce(
    (acc, sq) => acc + sq.results.length,
    0,
  );

  // Clamp active tab index.
  const safeTabIndex = Math.min(activeTabIndex, subQuestions.length - 1);
  const activeTab = subQuestions[safeTabIndex >= 0 ? safeTabIndex : 0];

  return (
    <Box
      flexDirection="column"
      borderStyle="round"
      borderColor={effectiveBorderColor}
      paddingX={1}
    >
      {/* Title bar */}
      <TitleBar
        intent={intent}
        durationMs={durationMs}
        cached={cached}
        expanded={expanded}
        borderColor={effectiveBorderColor}
        cacheColor={colors.knowledgeCard.cache}
      />

      {/* Collapsed: summary line */}
      {!expanded && (
        <Box>
          <Text color={colors.text.primary}>
            {totalResults} results
          </Text>
          <Text color={colors.text.secondary}>
            {' '}{'·'} {subQuestions.length} sub-queries
          </Text>
        </Box>
      )}

      {/* Expanded content */}
      {expanded && (
        <Box flexDirection="column" marginTop={0}>
          {/* Tab bar (only if multiple sub-questions) */}
          {subQuestions.length > 1 && (
            <Box flexDirection="column">
              <TabBar
                tabs={subQuestions}
                activeIndex={safeTabIndex}
                activeColor={colors.knowledgeCard.tabActive}
                inactiveColor={colors.knowledgeCard.tabInactive}
              />
              <Text>{' '}</Text>
            </Box>
          )}

          {/* Results list for active tab */}
          {activeTab != null && activeTab.results.slice(0, MAX_RESULTS_VISIBLE).map((result, i) => (
            <Box key={result.chunkId || i} flexDirection="column" marginBottom={1}>
              <ResultRow
                result={result}
                scoreColor={colors.knowledgeCard.score}
                citationColor={colors.knowledgeCard.citation}
                graphColor={colors.knowledgeCard.graph}
                textColor={colors.text.primary}
              />
            </Box>
          ))}

          {/* Synthesized answer */}
          {answer !== '' && (
            <Box flexDirection="column" marginTop={1}>
              <Text color={colors.knowledgeCard.answerDivider} bold>
                ── Answer ──
              </Text>
              {answer.split('\n').slice(0, MAX_ANSWER_LINES).map((line, i) => (
                <Text key={i} color={colors.text.primary}>
                  {line}
                </Text>
              ))}
            </Box>
          )}
        </Box>
      )}
    </Box>
  );
}
