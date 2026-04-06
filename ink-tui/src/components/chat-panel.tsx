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

import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Box, Text } from 'ink';
import type { ChatMessage } from '../types/chat.js';
import { MessageList } from './message-list.js';
import { useUIState, useUIActions } from '../contexts/ui-state-context.js';
import { useChatActions } from '../contexts/chat-context.js';
import { useTheme } from '../contexts/theme-context.js';
import { useKeypress } from '../hooks/use-keypress.js';
import { Command } from '../key/commands.js';
import { useEngine } from '../engine/engine-context.js';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface ChatPanelProps {
  /** The full message array from the engine/state. */
  messages: readonly ChatMessage[];
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function ChatPanel({ messages }: ChatPanelProps): React.ReactElement {
  const { colors } = useTheme();
  const uiState = useUIState();
  const actions = useUIActions();
  const chatActions = useChatActions();
  const engine = useEngine();

  const {
    scrollMode,
    autoScroll,
    focusedMessageIndex,
  } = uiState;

  // Track visible height for page-scroll calculations
  const [visibleHeight, setVisibleHeight] = useState(20);
  const containerRef = useRef<{ nodeRef?: { current?: { offsetHeight?: number } } }>(null);

  // --- Scroll mode keypress handler (priority 5) ---
  const handleScrollKey = useCallback(
    (input: string, _key: unknown, command: Command | undefined): boolean => {
      if (!scrollMode) return false;

      switch (command) {
        // k/Up arrow — move up. Up arrow resolves to HISTORY_UP (earlier
        // in enum) rather than SCROLL_UP, so accept both.
        case Command.SCROLL_UP:
        case Command.HISTORY_UP: {
          const newIndex = Math.max(0, focusedMessageIndex - 1);
          actions.setFocusedMessage(newIndex);
          return true;
        }

        // j/Down arrow — move down. Down arrow resolves to HISTORY_DOWN
        // (earlier in enum) rather than SCROLL_DOWN, so accept both.
        case Command.SCROLL_DOWN:
        case Command.HISTORY_DOWN: {
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

        case Command.EXIT_SCROLL_MODE:
        case Command.ENTER_SCROLL_MODE: {
          // Both resolve to Escape — when already in scroll mode, exit it.
          // The command resolver may map Escape to ENTER_SCROLL_MODE
          // (first in enum order) even when we need EXIT_SCROLL_MODE.
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

        // Toggle expand on focused message (Enter)
        // The resolver maps Return to SUBMIT (first in enum), so we
        // also accept SUBMIT here when in scroll mode.
        case Command.TOGGLE_EXPAND:
        case Command.SUBMIT: {
          const focused = messages[focusedMessageIndex];
          if (focused) {
            if (focused.kind === 'tool_call') {
              chatActions.toggleToolCardExpand(focused.toolId);
            } else if (focused.kind === 'kb_result') {
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

        // KB feedback — only consume the key when focused on a KB card,
        // otherwise fall through so 'y' can still copy to clipboard.
        case Command.FEEDBACK_POSITIVE: {
          const focused = messages[focusedMessageIndex];
          if (focused && focused.kind === 'kb_result') {
            engine.sendKbFeedback(focused.queryId, 'positive');
            return true;
          }
          return false;
        }

        case Command.FEEDBACK_NEGATIVE: {
          const focused = messages[focusedMessageIndex];
          if (focused && focused.kind === 'kb_result') {
            engine.sendKbFeedback(focused.queryId, 'negative');
            return true;
          }
          return false;
        }

        // '/' — exit scroll mode and let the character fall through to the
        // input handler so it inserts '/' and triggers autocomplete.
        case Command.START_SEARCH: {
          actions.exitScrollMode();
          return false; // don't consume — input handler will insert '/'
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
        if (focused.kind === 'user') textToCopy = focused.text;
        else if (focused.kind === 'assistant') textToCopy = focused.content;
        else if (focused.kind === 'system') textToCopy = focused.text;
        else if (focused.kind === 'tool_call') textToCopy = focused.output || focused.inputPreview;
        else if (focused.kind === 'kb_result') textToCopy = focused.answer ?? focused.query;

        if (textToCopy) {
          // Dynamic import to avoid top-level async in component
          import('clipboardy').then((clip) => {
            clip.default.writeSync(textToCopy);
            actions.showToast('Copied to clipboard');
          }).catch(() => {
            actions.showToast('Clipboard not available', 2000, 'warning');
          });
        }
      }
      return true;
    }

    return false;
    },
    [scrollMode, focusedMessageIndex, messages, actions, chatActions, engine, visibleHeight],
  );

  useKeypress(handleScrollKey, {
    isActive: scrollMode,
    priority: 5,
  });

  // --- Enter scroll mode handler (priority 0, global) ---
  const handleEnterScrollMode = useCallback(
    (_input: string, _key: unknown, command: Command | undefined): boolean => {
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
      // PgUp/PgDn outside scroll mode — enter scroll mode and page
      if (command === Command.SCROLL_UP_PAGE) {
        actions.enterScrollMode();
        const target = Math.max(0, messages.length - 1 - Math.max(1, visibleHeight - 2));
        actions.setFocusedMessage(target);
        return true;
      }
      if (command === Command.SCROLL_DOWN_PAGE) {
        // If not in scroll mode, PgDn at the bottom is a no-op
        return false;
      }

      return false;
    },
    [actions, messages.length, visibleHeight],
  );

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

  return (
    <Box flexDirection="column" flexGrow={1} overflow="hidden">
      {/* Scroll mode indicator */}
      {showScrollIndicator && (
        <Box height={1} flexShrink={0}>
          <Text color={colors.status.warning} bold>
            {' SCROLL '}
          </Text>
          <Text color={colors.text.secondary} dimColor>
            j/k:nav  g/G:top/bottom  Esc:back
          </Text>
        </Box>
      )}

      {/* All messages — rendered directly (no <Static> to avoid banner timing issues) */}
      {messages.length > 0 && (
        <MessageList
          messages={messages}
          focusedIndex={scrollMode ? focusedMessageIndex : -1}
        />
      )}

      {/* Empty state */}
      {messages.length === 0 && (
        <Box flexDirection="column" flexGrow={1} justifyContent="center" alignItems="center">
          <Text color={colors.text.secondary} dimColor>
            Type a message to get started
          </Text>
        </Box>
      )}
    </Box>
  );
}
