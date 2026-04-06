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
import type { KbChunkResult } from '../types/messages.js';
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
export declare function KnowledgeCard({ queryId, subQuestions, answer, cached, graphExpanded, durationMs, expanded, activeTabIndex, onToggleExpand, onTabChange, isFocused, }: KnowledgeCardProps): React.ReactElement;
