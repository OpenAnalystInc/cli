/**
 * ToolCard — inline bordered tool call card rendered inside the chat.
 *
 * Mirrors the Rust tui-widgets/tool_card.rs widget:
 *   - Rounded border, color by status (running=brand blue, completed=dim, failed=red)
 *   - Title line: spinner/check/cross + tool name + elapsed time
 *   - Input preview (first line, truncated)
 *   - Expanded: separator + output lines (max 20, with overflow indicator)
 *   - Optional DiffView for Edit/Write tools
 *
 * All colors from useTheme() semantic tokens.
 */
import React from 'react';
import type { DiffInfo } from '../types/messages.js';
export interface ToolCardProps {
    /** Unique tool call ID. */
    toolId: string;
    /** Tool name — e.g. "Bash", "Read", "Edit", "Write". */
    toolName: string;
    /** Execution status. */
    status: 'running' | 'completed' | 'failed';
    /** Tool input preview string. */
    input: string;
    /** Tool output (populated after completion). */
    output?: string;
    /** Execution duration in milliseconds. */
    durationMs?: number;
    /** Structured diff data for Edit/Write tools. */
    diff?: DiffInfo;
    /** Whether the output section is expanded. */
    expanded: boolean;
    /** Callback to toggle expand/collapse. */
    onToggleExpand: () => void;
    /** Whether this card is focused in scroll mode. */
    isFocused: boolean;
}
export declare function ToolCard({ toolId, toolName, status, input, output, durationMs, diff, expanded, onToggleExpand, isFocused, }: ToolCardProps): React.ReactElement;
