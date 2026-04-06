import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
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
import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Box, Static, Text } from 'ink';
import { MessageList } from './message-list.js';
import { useUIState, useUIActions } from '../contexts/ui-state-context.js';
import { useChatActions } from '../contexts/chat-context.js';
import { useTheme } from '../contexts/theme-context.js';
import { useKeypress } from '../hooks/use-keypress.js';
import { Command } from '../key/commands.js';
import { useEngine } from '../engine/engine-context.js';
// ---------------------------------------------------------------------------
// Internal: find the last streaming message index
// ---------------------------------------------------------------------------
function findStreamingBoundary(messages) {
    for (let i = messages.length - 1; i >= 0; i--) {
        const msg = messages[i];
        if (msg.kind === 'assistant' && msg.streaming) {
            // Everything before this index can be treated as "static"
            return i;
        }
        if (msg.kind === 'tool_call' && msg.status === 'running') {
            return i;
        }
    }
    // No streaming message — everything is static
    return messages.length;
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function ChatPanel({ messages }) {
    const { colors } = useTheme();
    const uiState = useUIState();
    const actions = useUIActions();
    const chatActions = useChatActions();
    const engine = useEngine();
    const { scrollMode, autoScroll, focusedMessageIndex, } = uiState;
    // Track visible height for page-scroll calculations
    const [visibleHeight, setVisibleHeight] = useState(20);
    const containerRef = useRef(null);
    // Split messages into static (fully rendered) and dynamic (streaming/active)
    const streamingBoundary = useMemo(() => findStreamingBoundary(messages), [messages]);
    const staticMessages = useMemo(() => messages.slice(0, streamingBoundary), [messages, streamingBoundary]);
    const dynamicMessages = useMemo(() => messages.slice(streamingBoundary), [messages, streamingBoundary]);
    // --- Scroll mode keypress handler (priority 5) ---
    const handleScrollKey = useCallback((input, _key, command) => {
        if (!scrollMode)
            return false;
        switch (command) {
            case Command.SCROLL_UP: {
                const newIndex = Math.max(0, focusedMessageIndex - 1);
                actions.setFocusedMessage(newIndex);
                return true;
            }
            case Command.SCROLL_DOWN: {
                const newIndex = Math.min(messages.length - 1, focusedMessageIndex + 1);
                actions.setFocusedMessage(newIndex);
                return true;
            }
            case Command.JUMP_TO_TOP: {
                actions.setFocusedMessage(0);
                return true;
            }
            case Command.JUMP_TO_BOTTOM: {
                actions.setFocusedMessage(messages.length - 1);
                return true;
            }
            case Command.EXIT_SCROLL_MODE: {
                actions.exitScrollMode();
                return true;
            }
            case Command.SCROLL_UP_PAGE: {
                const pageSize = Math.max(1, visibleHeight - 2);
                const newIndex = Math.max(0, focusedMessageIndex - pageSize);
                actions.setFocusedMessage(newIndex);
                return true;
            }
            case Command.SCROLL_DOWN_PAGE: {
                const pageSize = Math.max(1, visibleHeight - 2);
                const newIndex = Math.min(messages.length - 1, focusedMessageIndex + pageSize);
                actions.setFocusedMessage(newIndex);
                return true;
            }
            // Toggle expand on focused message (Enter or Space)
            case Command.TOGGLE_EXPAND: {
                const focused = messages[focusedMessageIndex];
                if (focused) {
                    if (focused.kind === 'tool_call') {
                        chatActions.toggleToolCardExpand(focused.toolId);
                    }
                    else if (focused.kind === 'kb_result') {
                        chatActions.toggleKBExpand(focused.id);
                    }
                }
                return true;
            }
            // Tab navigation for KB cards
            case Command.NEXT_TAB: {
                const focused = messages[focusedMessageIndex];
                if (focused && focused.kind === 'kb_result') {
                    chatActions.setKBActiveTab(focused.id, focused.activeTab + 1);
                }
                return true;
            }
            case Command.PREV_TAB: {
                const focused = messages[focusedMessageIndex];
                if (focused && focused.kind === 'kb_result') {
                    chatActions.setKBActiveTab(focused.id, Math.max(0, focused.activeTab - 1));
                }
                return true;
            }
            // KB feedback
            case Command.FEEDBACK_POSITIVE: {
                const focused = messages[focusedMessageIndex];
                if (focused && focused.kind === 'kb_result') {
                    engine.sendKbFeedback(focused.queryId, 'positive');
                }
                return true;
            }
            case Command.FEEDBACK_NEGATIVE: {
                const focused = messages[focusedMessageIndex];
                if (focused && focused.kind === 'kb_result') {
                    engine.sendKbFeedback(focused.queryId, 'negative');
                }
                return true;
            }
            default:
                break;
        }
        // Also handle raw j/k for vim-style navigation even if command
        // resolution didn't fire (fallback)
        if (input === 'j') {
            const newIndex = Math.min(messages.length - 1, focusedMessageIndex + 1);
            actions.setFocusedMessage(newIndex);
            return true;
        }
        if (input === 'k') {
            const newIndex = Math.max(0, focusedMessageIndex - 1);
            actions.setFocusedMessage(newIndex);
            return true;
        }
        if (input === 'G') {
            actions.setFocusedMessage(messages.length - 1);
            return true;
        }
        if (input === 'g') {
            // gg = top (simplified: single g goes to top)
            actions.setFocusedMessage(0);
            return true;
        }
        // 'y' = copy focused message to clipboard
        if (input === 'y') {
            const focused = messages[focusedMessageIndex];
            if (focused) {
                let textToCopy = '';
                if (focused.kind === 'user')
                    textToCopy = focused.text;
                else if (focused.kind === 'assistant')
                    textToCopy = focused.content;
                else if (focused.kind === 'system')
                    textToCopy = focused.text;
                else if (focused.kind === 'tool_call')
                    textToCopy = focused.output || focused.inputPreview;
                else if (focused.kind === 'kb_result')
                    textToCopy = focused.answer ?? focused.query;
                if (textToCopy) {
                    // Dynamic import to avoid top-level async in component
                    import('clipboardy').then((clip) => {
                        clip.default.writeSync(textToCopy);
                    }).catch(() => {
                        // Silently fail if clipboard not available
                    });
                    // Brief notification via status bar message
                    actions.setPhase('done', 'Copied!');
                    setTimeout(() => {
                        actions.setPhase('idle');
                    }, 1500);
                }
            }
            return true;
        }
        return false;
    }, [scrollMode, focusedMessageIndex, messages, actions, chatActions, engine, visibleHeight]);
    useKeypress(handleScrollKey, {
        isActive: scrollMode,
        priority: 5,
    });
    // --- Enter scroll mode handler (priority 0, global) ---
    const handleEnterScrollMode = useCallback((_input, _key, command) => {
        if (command === Command.ENTER_SCROLL_MODE) {
            actions.enterScrollMode();
            // Focus the last message
            if (messages.length > 0) {
                actions.setFocusedMessage(messages.length - 1);
            }
            return true;
        }
        // Global scroll commands (Ctrl+Home, Ctrl+End, PgUp, PgDn)
        if (command === Command.SCROLL_TO_TOP) {
            actions.enterScrollMode();
            actions.setFocusedMessage(0);
            return true;
        }
        if (command === Command.SCROLL_TO_BOTTOM) {
            actions.exitScrollMode();
            return true;
        }
        return false;
    }, [actions, messages.length]);
    useKeypress(handleEnterScrollMode, {
        isActive: !scrollMode && messages.length > 0,
        priority: 0,
    });
    // --- Auto-scroll: when new messages arrive and autoScroll is on ---
    useEffect(() => {
        if (autoScroll && messages.length > 0) {
            // Auto-scroll is handled by Ink's natural flow — new content
            // at the bottom pushes the view. We just need to ensure we
            // don't have a stale focusedMessageIndex.
        }
    }, [autoScroll, messages.length]);
    // --- Render ---
    const showScrollIndicator = scrollMode;
    return (_jsxs(Box, { flexDirection: "column", flexGrow: 1, paddingX: 1, overflow: "hidden", children: [showScrollIndicator && (_jsxs(Box, { height: 1, flexShrink: 0, children: [_jsx(Text, { color: colors.status.warning, bold: true, children: ' SCROLL ' }), _jsx(Text, { color: colors.text.secondary, dimColor: true, children: "j/k:nav  g/G:top/bottom  Esc:back" })] })), staticMessages.length > 0 && (_jsx(Static, { items: staticMessages, children: (msg) => (_jsx(Box, { flexDirection: "column", children: _jsx(MessageList, { messages: [msg], focusedIndex: scrollMode && focusedMessageIndex < streamingBoundary
                            ? focusedMessageIndex - messages.indexOf(msg)
                            : -1 }) }, `static-${msg.id}`)) })), dynamicMessages.length > 0 && (_jsx(MessageList, { messages: dynamicMessages, focusedIndex: scrollMode
                    ? focusedMessageIndex - streamingBoundary
                    : -1 })), messages.length === 0 && (_jsx(Box, { flexDirection: "column", flexGrow: 1, justifyContent: "center", alignItems: "center", children: _jsx(Text, { color: colors.text.secondary, dimColor: true, children: "Type a message to get started" }) }))] }));
}
//# sourceMappingURL=chat-panel.js.map