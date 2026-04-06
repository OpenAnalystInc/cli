/**
 * DiffView — renders unified diff hunks with colored +/- lines.
 *
 * Mirrors the Rust tool_card.rs diff rendering: green for added lines,
 * red for removed lines, dim for context lines, with hunk headers.
 *
 * All colors come from useTheme() semantic tokens — never hardcoded.
 */
import React from 'react';
import type { DiffHunk } from '../types/messages.js';
export interface DiffViewProps {
    /** File path being diffed. */
    filePath: string;
    /** Number of lines added across all hunks. */
    added: number;
    /** Number of lines removed across all hunks. */
    removed: number;
    /** Diff hunks to render. */
    hunks: DiffHunk[];
    /** Maximum total lines to show (across all hunks). Defaults to 20. */
    maxLines?: number;
    /** Optional className-style width constraint. */
    maxWidth?: number;
}
export declare function DiffView({ filePath, added, removed, hunks, maxLines, maxWidth, }: DiffViewProps): React.ReactElement;
