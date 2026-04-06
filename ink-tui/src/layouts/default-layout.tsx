/**
 * DefaultLayout — main layout component for the OpenAnalyst TUI.
 *
 * Panel arrangement (matching Rust crate tui::layout):
 *
 *   +-------------------------------+--------+
 *   |                               |        |
 *   |         Chat Panel            | Sidebar|
 *   |         (flex grow)           | (26ch) |
 *   |                               |        |
 *   +-------------------------------+--------+
 *   | Status bar (1 line)                     |
 *   +----------------------------------------+
 *   | Input box (5-10 lines dynamic)          |
 *   +----------------------------------------+
 *
 * Layout constraints from Rust:
 *   - SIDEBAR_WIDTH = 26
 *   - INPUT_MIN_HEIGHT = 5, INPUT_MAX_HEIGHT = 10
 *   - Sidebar hidden when terminal width < 60
 *   - Chat panel minimum width = 30
 */

import React, { useCallback, useRef, useEffect } from 'react';
import { Box, Text } from 'ink';
import { useUIState, useUIActions } from '../contexts/ui-state-context.js';
import { useTerminal } from '../contexts/terminal-context.js';
import { useTheme } from '../contexts/theme-context.js';
import { ChatPanel } from '../components/chat-panel.js';
import { useChatMessages, useChatActions } from '../contexts/chat-context.js';
import { InputBox } from '../components/input-box.js';
import { PermissionDialog } from '../components/permission-dialog.js';
import { AskUserDialog } from '../components/ask-user-dialog.js';
import { StatusBar } from '../components/status-bar.js';
import { Sidebar } from '../components/sidebar.js';
import { useEngine } from '../engine/engine-context.js';
import { useKeypress } from '../hooks/use-keypress.js';
import { Command } from '../key/commands.js';
import type { ActionCategory } from '../types/messages.js';

// ---------------------------------------------------------------------------
// Constants (matching Rust layout.rs)
// ---------------------------------------------------------------------------

/** Fixed sidebar width in columns. */
const SIDEBAR_WIDTH = 26;

/** Minimum input box height (including border). */
// const INPUT_MIN_HEIGHT = 5;

/** Maximum input box height (including border). */
// const INPUT_MAX_HEIGHT = 10;

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function DefaultLayout(): React.ReactElement {
  const uiState = useUIState();
  const uiActions = useUIActions();
  const chatActions = useChatActions();
  const terminal = useTerminal();
  const { colors } = useTheme();
  const messages = useChatMessages();
  const engine = useEngine();

  const { sidebarVisible, permissionDialog, askUserDialog, exitPending } = uiState;
  const showSidebar = sidebarVisible && terminal.canShowSidebar;

  // -- Exit pending timer --
  const exitTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (exitTimerRef.current) clearTimeout(exitTimerRef.current);
    };
  }, []);

  // -- Double Ctrl+C quit --
  const handleGlobalKeys = useCallback(
    (input: string, key: { ctrl?: boolean; escape?: boolean; meta?: boolean; shift?: boolean }, command: Command | undefined): boolean => {
      // Ctrl+C handling
      if (key.ctrl && input === 'c') {
        const isAgentRunning = uiState.phase !== 'idle' && uiState.phase !== 'done' && uiState.phase !== 'error';

        if (isAgentRunning) {
          // Cancel the running agent
          engine.cancelAgent();
          chatActions.pushSystem('Agent cancelled.', 'warning');
          return true;
        }

        if (exitPending) {
          // Second Ctrl+C — quit
          engine.quit();
          process.exit(0);
        }

        // First Ctrl+C while idle — set exit pending
        uiActions.setExitPending(true);
        if (exitTimerRef.current) clearTimeout(exitTimerRef.current);
        exitTimerRef.current = setTimeout(() => {
          uiActions.setExitPending(false);
        }, 2000);
        return true;
      }

      // Any other key resets exit pending
      if (exitPending) {
        uiActions.setExitPending(false);
        if (exitTimerRef.current) {
          clearTimeout(exitTimerRef.current);
          exitTimerRef.current = null;
        }
        // Don't consume — let the key pass through
      }

      // Ctrl+B: run in background
      // Primary handling is in input-box.tsx which captures current text.
      // This global handler only fires if the input box didn't consume it
      // (e.g. input was empty), so we do nothing here.
      if (command === Command.RUN_IN_BACKGROUND) {
        return false;
      }

      // Ctrl+P: cycle permission mode
      if (command === Command.CYCLE_PERMISSION_MODE) {
        uiActions.cyclePermissionMode();
        return true;
      }

      // F2 / Ctrl+E: toggle sidebar
      if (command === Command.TOGGLE_SIDEBAR) {
        uiActions.toggleSidebar();
        return true;
      }

      if (command === Command.FOCUS_SIDEBAR) {
        uiActions.focusSidebar();
        return true;
      }

      // Ctrl+L: clear chat
      if (command === Command.CLEAR_CHAT) {
        engine.clearChat();
        return true;
      }

      return false;
    },
    [uiState.phase, exitPending, engine, uiActions, chatActions],
  );

  useKeypress(handleGlobalKeys, {
    isActive: true,
    priority: 0,
  });

  // -- Double-Esc rapid press to cancel agent --
  const lastEscRef = useRef<number>(0);
  const handleDoubleEsc = useCallback(
    (_input: string, key: { escape?: boolean }, _command: Command | undefined): boolean => {
      if (!key.escape) return false;

      const now = Date.now();
      const isAgentRunning = uiState.phase !== 'idle' && uiState.phase !== 'done' && uiState.phase !== 'error';

      // If agent is running and double-Esc within 500ms, cancel
      if (isAgentRunning && now - lastEscRef.current < 500) {
        engine.cancelAgent();
        chatActions.pushSystem('Agent cancelled.', 'warning');
        lastEscRef.current = 0;
        return true;
      }

      // If idle and double-Esc within 1 second, undo last context file addition
      if (!isAgentRunning && now - lastEscRef.current < 1000) {
        if (uiState.contextFiles.length > 0) {
          const lastFile = uiState.contextFiles[uiState.contextFiles.length - 1]!;
          engine.bridge.toggleContextFile(lastFile, 'remove');
          uiActions.removeContextFile(lastFile);
          chatActions.pushSystem(`Removed context file: ${lastFile}`, 'info');
          lastEscRef.current = 0;
          return true;
        }
      }

      lastEscRef.current = now;
      return false; // Let single-Esc propagate to scroll mode
    },
    [uiState.phase, uiState.contextFiles, engine, uiActions, chatActions],
  );

  useKeypress(handleDoubleEsc, {
    isActive: true,
    priority: 1, // Just above global, below scroll/input handlers
  });

  // -- Sidebar engine callbacks --
  const handleRoutingChange = useCallback(
    (category: ActionCategory, tier: string) => {
      engine.bridge.changeRouting(category, tier as 'fast' | 'balanced' | 'capable');
    },
    [engine],
  );

  const handleFileToggle = useCallback(
    (path: string) => {
      // Check if already in context, toggle accordingly
      const isInContext = uiState.contextFiles.includes(path);
      engine.bridge.toggleContextFile(path, isInContext ? 'remove' : 'add');
      if (isInContext) {
        uiActions.removeContextFile(path);
      } else {
        uiActions.addContextFile(path);
      }
    },
    [engine, uiState.contextFiles, uiActions],
  );

  // -- Submit handler for InputBox --
  const handleSubmit = useCallback(
    (text: string) => {
      engine.submitPrompt(text);
    },
    [engine],
  );

  return (
    <Box
      flexDirection="column"
      width={terminal.width}
      height={terminal.height}
    >
      {/* ── Main content area: chat + optional sidebar ── */}
      <Box flexDirection="row" flexGrow={1}>
        {/* Chat panel takes remaining space */}
        <Box flexDirection="column" flexGrow={1} minWidth={30}>
          <ChatPanel messages={messages} />
        </Box>

        {/* Sidebar: 26 chars, right side, toggleable */}
        {showSidebar && (
          <Box
            width={SIDEBAR_WIDTH}
            flexShrink={0}
            flexDirection="column"
            borderStyle="single"
            borderColor={colors.border.default}
          >
            <Sidebar
              onRoutingChange={handleRoutingChange}
              onFileToggle={handleFileToggle}
            />
          </Box>
        )}
      </Box>

      {/* ── Status bar: 1 line ── */}
      <Box height={1} flexShrink={0}>
        {exitPending ? (
          <Box width="100%" justifyContent="center">
            <Text color={colors.status.warning} bold>
              Press Ctrl+C again to quit
            </Text>
          </Box>
        ) : (
          <StatusBar />
        )}
      </Box>

      {/* ── Input box: 5-10 lines dynamic ── */}
      <Box minHeight={5} flexShrink={0}>
        <InputBox onSubmit={handleSubmit} />
      </Box>

      {/* ── Modal overlays (rendered after main content, visually on top) ── */}
      {permissionDialog != null && <PermissionDialog />}
      {askUserDialog != null && <AskUserDialog />}
    </Box>
  );
}
