/**
 * ChatPanel — scrollable message list with auto-scroll and scroll mode.
 *
 * Auto-scroll behavior:
 *   - Stays at bottom during streaming (new content pushes view down)
 *   - Disables auto-scroll when user scrolls up manually
 *   - Re-enables on "jump to bottom" or when new user message is sent
 *
 * Scroll mode (Esc key):
 *   - j/k to navigate messages
 *   - Focused message gets a left border highlight
 *   - Esc again or Enter exits scroll mode
 *   - Sidebar auto-hides when scroll begins
 *
 * Uses Ink's <Static> for fully-rendered (non-streaming) messages
 * to optimize re-render performance.
 */
import React from 'react';
import type { ChatMessage } from '../types/chat.js';
export interface ChatPanelProps {
    /** The full message array from the engine/state. */
    messages: readonly ChatMessage[];
}
export declare function ChatPanel({ messages }: ChatPanelProps): React.ReactElement;
