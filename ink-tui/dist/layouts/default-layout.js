import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * DefaultLayout — main layout component for the OpenAnalyst TUI.
 *
 * Layout:
 *   - Chat panel takes full width when sidebar is hidden
 *   - F2 toggles sidebar as a right column (26 chars)
 *   - Sidebar auto-hides when task starts, can be reopened with F2
 *   - Status bar + input box always full width at bottom
 */
import { useCallback, useRef, useEffect } from 'react';
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
import { ToastDisplay } from '../components/toast-display.js';
import { Sidebar } from '../components/sidebar.js';
import { useEngine } from '../engine/engine-context.js';
import { useKeypress } from '../hooks/use-keypress.js';
import { Command } from '../key/commands.js';
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
const SIDEBAR_WIDTH = 28;
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function DefaultLayout() {
    const uiState = useUIState();
    const uiActions = useUIActions();
    const chatActions = useChatActions();
    const terminal = useTerminal();
    const { colors } = useTheme();
    const messages = useChatMessages();
    const engine = useEngine();
    const { sidebarVisible, permissionDialog, askUserDialog, exitPending } = uiState;
    const showSidebar = sidebarVisible && terminal.canShowSidebar;
    const exitTimerRef = useRef(null);
    useEffect(() => {
        return () => {
            if (exitTimerRef.current)
                clearTimeout(exitTimerRef.current);
        };
    }, []);
    // -- Global keybindings --
    const handleGlobalKeys = useCallback((input, key, command) => {
        if (key.ctrl && input === 'c') {
            const isAgentRunning = uiState.phase !== 'idle' && uiState.phase !== 'done' && uiState.phase !== 'error';
            if (isAgentRunning) {
                engine.cancelAgent();
                chatActions.pushSystem('Agent cancelled.', 'warning');
                return true;
            }
            if (exitPending) {
                engine.quit();
                process.exit(0);
            }
            uiActions.setExitPending(true);
            if (exitTimerRef.current)
                clearTimeout(exitTimerRef.current);
            exitTimerRef.current = setTimeout(() => {
                uiActions.setExitPending(false);
            }, 2000);
            return true;
        }
        if (exitPending) {
            uiActions.setExitPending(false);
            if (exitTimerRef.current) {
                clearTimeout(exitTimerRef.current);
                exitTimerRef.current = null;
            }
        }
        if (command === Command.RUN_IN_BACKGROUND)
            return false;
        if (command === Command.CYCLE_PERMISSION_MODE) {
            const modeLabels = {
                'prompt': 'Plan (read-only)',
                'read-only': 'Accept Edits',
                'workspace-write': 'Danger (full access)',
                'danger-full-access': 'Default (prompt)',
            };
            const nextLabel = modeLabels[uiState.permissionMode] ?? 'Default';
            uiActions.cyclePermissionMode();
            uiActions.showToast(`Permission mode: ${nextLabel}`, 2000);
            return true;
        }
        if (command === Command.TOGGLE_SIDEBAR || command === Command.FOCUS_SIDEBAR) {
            uiActions.toggleSidebar();
            return true;
        }
        if (key.ctrl && input === 'e') {
            uiActions.toggleSidebar();
            return true;
        }
        if (command === Command.CLEAR_CHAT) {
            engine.clearChat();
            uiActions.showToast('Chat cleared', 1500);
            return true;
        }
        return false;
    }, [uiState.phase, exitPending, engine, uiActions, chatActions]);
    useKeypress(handleGlobalKeys, { isActive: true, priority: 0 });
    // -- Esc: stop execution if running, otherwise scroll mode --
    const lastEscRef = useRef(0);
    const handleEsc = useCallback((_input, key, _command) => {
        if (!key.escape)
            return false;
        const isAgentRunning = uiState.phase !== 'idle' && uiState.phase !== 'done' && uiState.phase !== 'error';
        // If agent is running, single Esc stops it
        if (isAgentRunning) {
            engine.cancelAgent();
            chatActions.pushSystem('Agent stopped.', 'warning');
            return true;
        }
        // If idle, double-Esc removes last context file
        const now = Date.now();
        if (now - lastEscRef.current < 1000) {
            if (uiState.contextFiles.length > 0) {
                const lastFile = uiState.contextFiles[uiState.contextFiles.length - 1];
                engine.bridge.toggleContextFile(lastFile, 'remove');
                uiActions.removeContextFile(lastFile);
                chatActions.pushSystem(`Removed context file: ${lastFile}`, 'info');
                lastEscRef.current = 0;
                return true;
            }
        }
        lastEscRef.current = now;
        // Let Esc propagate to scroll mode handler
        return false;
    }, [uiState.phase, uiState.contextFiles, engine, uiActions, chatActions]);
    useKeypress(handleEsc, { isActive: true, priority: 1 });
    // -- Sidebar callbacks --
    const handleRoutingChange = useCallback((category, tier) => {
        engine.bridge.changeRouting(category, tier);
    }, [engine]);
    const handleFileToggle = useCallback((path) => {
        const isInContext = uiState.contextFiles.includes(path);
        engine.bridge.toggleContextFile(path, isInContext ? 'remove' : 'add');
        if (isInContext) {
            uiActions.removeContextFile(path);
            const basename = path.split(/[/\\]/).pop() ?? path;
            uiActions.showToast(`Removed: ${basename}`, 1500);
        }
        else {
            uiActions.addContextFile(path);
            const basename = path.split(/[/\\]/).pop() ?? path;
            uiActions.showToast(`Added: ${basename}`, 1500);
        }
    }, [engine, uiState.contextFiles, uiActions]);
    const handleSubmit = useCallback((text) => {
        engine.submitPrompt(text);
    }, [engine]);
    return (_jsxs(Box, { flexDirection: "column", width: terminal.width, children: [_jsxs(Box, { flexDirection: "row", children: [_jsx(Box, { flexDirection: "column", flexGrow: 1, minWidth: 30, children: _jsx(ChatPanel, { messages: messages }) }), showSidebar && (_jsx(Box, { width: SIDEBAR_WIDTH, flexShrink: 0, flexDirection: "column", borderStyle: "round", borderColor: uiState.sidebarFocused ? colors.status.warning : colors.border.default, overflow: "hidden", children: _jsx(Sidebar, { agents: uiState.sidebarAgents, files: uiState.sidebarFiles, plans: uiState.sidebarPlans, routing: uiState.sidebarRouting, activity: uiState.sidebarActivity, onRoutingChange: handleRoutingChange, onFileToggle: handleFileToggle }) }))] }), uiState.toastMessage != null && (_jsx(Box, { height: 1, flexShrink: 0, children: _jsx(ToastDisplay, {}) })), _jsx(Box, { height: 1, flexShrink: 0, children: exitPending ? (_jsx(Box, { width: "100%", justifyContent: "center", children: _jsx(Text, { color: colors.status.warning, bold: true, children: "Press Ctrl+C again to quit" }) })) : (_jsx(StatusBar, {})) }), _jsx(Box, { minHeight: 5, flexShrink: 0, children: _jsx(InputBox, { onSubmit: handleSubmit }) }), permissionDialog != null && _jsx(PermissionDialog, {}), askUserDialog != null && _jsx(AskUserDialog, {})] }));
}
//# sourceMappingURL=default-layout.js.map