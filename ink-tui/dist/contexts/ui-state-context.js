import { jsx as _jsx } from "react/jsx-runtime";
/**
 * UIStateProvider — central UI state for the entire TUI.
 *
 * Split into two contexts for performance:
 * - UIStateContext — the state object (changes frequently)
 * - UIActionsContext — stable action functions (never change identity)
 *
 * Components that only need to dispatch actions (e.g. keybinding handlers)
 * subscribe to UIActionsContext and avoid re-rendering on every state tick.
 */
import { createContext, useContext, useMemo, useReducer, } from 'react';
// ═══════════════════════════════════════════════════════════════════════════
// Initial state
// ═══════════════════════════════════════════════════════════════════════════
const PERMISSION_MODE_CYCLE = [
    'prompt',
    'read-only',
    'workspace-write',
    'danger-full-access',
];
function createInitialState() {
    return {
        mode: 'idle',
        permissionMode: 'prompt',
        sidebarVisible: false,
        sidebarFocused: false,
        scrollMode: false,
        autoScroll: true,
        scrollOffset: 0,
        focusedMessageIndex: -1,
        permissionDialog: null,
        askUserDialog: null,
        autocompleteVisible: false,
        autocompleteItems: [],
        autocompleteIndex: 0,
        voiceRecording: false,
        phase: 'idle',
        phaseLabel: '',
        elapsedMs: 0,
        tokensRemaining: null,
        inputMode: 'ready',
        inputLabel: '',
        currentModel: '',
        currentBranch: '',
        activeAgent: null,
        contextFiles: [],
        creditBalance: null,
        mcpServerCount: 0,
        terminalWidth: process.stdout.columns ?? 80,
        terminalHeight: process.stdout.rows ?? 24,
        exitPending: false,
    };
}
function uiReducer(state, action) {
    switch (action.type) {
        case 'TOGGLE_SIDEBAR': {
            if (state.sidebarFocused) {
                // Focused -> hide
                return { ...state, sidebarVisible: false, sidebarFocused: false, mode: 'idle' };
            }
            // Toggle visibility
            return { ...state, sidebarVisible: !state.sidebarVisible };
        }
        case 'FOCUS_SIDEBAR': {
            if (!state.sidebarVisible) {
                // Hidden -> show
                return { ...state, sidebarVisible: true, sidebarFocused: false };
            }
            if (!state.sidebarFocused) {
                // Visible but not focused -> focus
                return { ...state, sidebarFocused: true, mode: 'sidebar_focused' };
            }
            // Focused -> hide
            return { ...state, sidebarVisible: false, sidebarFocused: false, mode: 'idle' };
        }
        case 'HIDE_SIDEBAR':
            return { ...state, sidebarVisible: false, sidebarFocused: false, mode: state.mode === 'sidebar_focused' ? 'idle' : state.mode };
        case 'ENTER_SCROLL_MODE':
            return {
                ...state,
                scrollMode: true,
                autoScroll: false,
                mode: 'scroll',
                // Auto-hide sidebar when entering scroll mode
                sidebarVisible: false,
                sidebarFocused: false,
            };
        case 'EXIT_SCROLL_MODE':
            return {
                ...state,
                scrollMode: false,
                autoScroll: true,
                scrollOffset: 0,
                focusedMessageIndex: -1,
                mode: 'idle',
            };
        case 'CYCLE_PERMISSION_MODE': {
            const currentIndex = PERMISSION_MODE_CYCLE.indexOf(state.permissionMode);
            const nextIndex = (currentIndex + 1) % PERMISSION_MODE_CYCLE.length;
            return { ...state, permissionMode: PERMISSION_MODE_CYCLE[nextIndex] };
        }
        case 'SET_PHASE':
            return {
                ...state,
                phase: action.phase,
                phaseLabel: action.label ?? '',
                // Transition mode based on phase
                mode: action.phase === 'idle' || action.phase === 'done' || action.phase === 'error'
                    ? (state.scrollMode ? 'scroll' : 'idle')
                    : (state.scrollMode ? 'scroll' : 'streaming'),
                inputMode: action.phase === 'idle' || action.phase === 'done' || action.phase === 'error'
                    ? 'ready'
                    : state.inputMode === 'ready' ? 'agent_running' : state.inputMode,
            };
        case 'SET_ELAPSED':
            return { ...state, elapsedMs: action.elapsedMs };
        case 'SET_TOKENS_REMAINING':
            return { ...state, tokensRemaining: action.tokens };
        case 'SHOW_PERMISSION_DIALOG':
            return { ...state, permissionDialog: action.state };
        case 'DISMISS_PERMISSION_DIALOG':
            return { ...state, permissionDialog: null };
        case 'SHOW_ASK_USER_DIALOG':
            return { ...state, askUserDialog: action.state };
        case 'DISMISS_ASK_USER_DIALOG':
            return { ...state, askUserDialog: null };
        case 'SET_AUTO_SCROLL':
            return { ...state, autoScroll: action.enabled };
        case 'SCROLL_BY':
            return {
                ...state,
                scrollOffset: Math.max(0, state.scrollOffset + action.delta),
                autoScroll: false,
                // Auto-hide sidebar on scroll
                sidebarVisible: false,
                sidebarFocused: false,
                mode: 'scroll',
                scrollMode: true,
            };
        case 'SCROLL_TO':
            return { ...state, scrollOffset: Math.max(0, action.offset) };
        case 'SET_FOCUSED_MESSAGE':
            return { ...state, focusedMessageIndex: action.index };
        case 'ADD_CONTEXT_FILE':
            if (state.contextFiles.includes(action.path))
                return state;
            return { ...state, contextFiles: [...state.contextFiles, action.path] };
        case 'REMOVE_CONTEXT_FILE':
            return { ...state, contextFiles: state.contextFiles.filter((f) => f !== action.path) };
        case 'SET_CONTEXT_FILES':
            return { ...state, contextFiles: action.files };
        case 'SET_TERMINAL_SIZE': {
            const canShowSidebar = action.width >= 60;
            return {
                ...state,
                terminalWidth: action.width,
                terminalHeight: action.height,
                // Auto-hide sidebar if terminal becomes too narrow
                sidebarVisible: canShowSidebar ? state.sidebarVisible : false,
                sidebarFocused: canShowSidebar ? state.sidebarFocused : false,
            };
        }
        case 'SET_EXIT_PENDING':
            return { ...state, exitPending: action.pending };
        case 'SET_MODE':
            return { ...state, mode: action.mode };
        case 'SET_INPUT_MODE':
            return { ...state, inputMode: action.inputMode, inputLabel: action.label ?? state.inputLabel };
        case 'SET_MODEL_INFO':
            return { ...state, currentModel: action.model };
        case 'SET_BRANCH':
            return { ...state, currentBranch: action.branch };
        case 'SET_ACTIVE_AGENT':
            return { ...state, activeAgent: action.agentId };
        case 'SET_CREDIT_BALANCE':
            return { ...state, creditBalance: action.balance };
        case 'SET_MCP_SERVER_COUNT':
            return { ...state, mcpServerCount: action.count };
        case 'SET_AUTOCOMPLETE':
            return {
                ...state,
                autocompleteVisible: action.visible,
                autocompleteItems: action.items ?? state.autocompleteItems,
                autocompleteIndex: action.index ?? 0,
            };
        case 'SET_VOICE_RECORDING':
            return {
                ...state,
                voiceRecording: action.recording,
                mode: action.recording ? 'voice_recording' : 'idle',
            };
        case 'CLEAR_CHAT':
            return {
                ...state,
                scrollOffset: 0,
                focusedMessageIndex: -1,
                autoScroll: true,
                scrollMode: false,
                mode: 'idle',
            };
        default:
            return state;
    }
}
// ═══════════════════════════════════════════════════════════════════════════
// Contexts
// ═══════════════════════════════════════════════════════════════════════════
const UIStateContext = createContext(null);
const UIActionsContext = createContext(null);
export function UIStateProvider({ children }) {
    const [state, dispatch] = useReducer(uiReducer, undefined, createInitialState);
    // Build stable action object — dispatch never changes, so these are stable.
    const actions = useMemo(() => ({
        toggleSidebar: () => dispatch({ type: 'TOGGLE_SIDEBAR' }),
        focusSidebar: () => dispatch({ type: 'FOCUS_SIDEBAR' }),
        hideSidebar: () => dispatch({ type: 'HIDE_SIDEBAR' }),
        enterScrollMode: () => dispatch({ type: 'ENTER_SCROLL_MODE' }),
        exitScrollMode: () => dispatch({ type: 'EXIT_SCROLL_MODE' }),
        cyclePermissionMode: () => dispatch({ type: 'CYCLE_PERMISSION_MODE' }),
        setPhase: (phase, label) => dispatch({ type: 'SET_PHASE', phase, label }),
        setElapsed: (elapsedMs) => dispatch({ type: 'SET_ELAPSED', elapsedMs }),
        setTokensRemaining: (tokens) => dispatch({ type: 'SET_TOKENS_REMAINING', tokens }),
        showPermissionDialog: (state) => dispatch({ type: 'SHOW_PERMISSION_DIALOG', state }),
        dismissPermissionDialog: () => dispatch({ type: 'DISMISS_PERMISSION_DIALOG' }),
        showAskUserDialog: (state) => dispatch({ type: 'SHOW_ASK_USER_DIALOG', state }),
        dismissAskUserDialog: () => dispatch({ type: 'DISMISS_ASK_USER_DIALOG' }),
        setAutoScroll: (enabled) => dispatch({ type: 'SET_AUTO_SCROLL', enabled }),
        scrollBy: (delta) => dispatch({ type: 'SCROLL_BY', delta }),
        scrollTo: (offset) => dispatch({ type: 'SCROLL_TO', offset }),
        setFocusedMessage: (index) => dispatch({ type: 'SET_FOCUSED_MESSAGE', index }),
        addContextFile: (path) => dispatch({ type: 'ADD_CONTEXT_FILE', path }),
        removeContextFile: (path) => dispatch({ type: 'REMOVE_CONTEXT_FILE', path }),
        setContextFiles: (files) => dispatch({ type: 'SET_CONTEXT_FILES', files }),
        setTerminalSize: (width, height) => dispatch({ type: 'SET_TERMINAL_SIZE', width, height }),
        setExitPending: (pending) => dispatch({ type: 'SET_EXIT_PENDING', pending }),
        setMode: (mode) => dispatch({ type: 'SET_MODE', mode }),
        setInputMode: (inputMode, label) => dispatch({ type: 'SET_INPUT_MODE', inputMode, label }),
        setModelInfo: (model) => dispatch({ type: 'SET_MODEL_INFO', model }),
        setBranch: (branch) => dispatch({ type: 'SET_BRANCH', branch }),
        setActiveAgent: (agentId) => dispatch({ type: 'SET_ACTIVE_AGENT', agentId }),
        setCreditBalance: (balance) => dispatch({ type: 'SET_CREDIT_BALANCE', balance }),
        setMcpServerCount: (count) => dispatch({ type: 'SET_MCP_SERVER_COUNT', count }),
        setAutocomplete: (visible, items, index) => dispatch({ type: 'SET_AUTOCOMPLETE', visible, items, index }),
        setVoiceRecording: (recording) => dispatch({ type: 'SET_VOICE_RECORDING', recording }),
        clearChat: () => dispatch({ type: 'CLEAR_CHAT' }),
    }), []);
    return (_jsx(UIStateContext.Provider, { value: state, children: _jsx(UIActionsContext.Provider, { value: actions, children: children }) }));
}
// ═══════════════════════════════════════════════════════════════════════════
// Consumer hooks
// ═══════════════════════════════════════════════════════════════════════════
/**
 * Returns the current UI state. Re-renders on every state change.
 * Use `useUIActions()` if you only need to dispatch.
 */
export function useUIState() {
    const ctx = useContext(UIStateContext);
    if (!ctx) {
        throw new Error('useUIState() must be used within a <UIStateProvider>');
    }
    return ctx;
}
/**
 * Returns stable action dispatchers. Never causes re-renders.
 * Prefer this over `useUIState()` in event handlers and keybinding subscribers.
 */
export function useUIActions() {
    const ctx = useContext(UIActionsContext);
    if (!ctx) {
        throw new Error('useUIActions() must be used within a <UIStateProvider>');
    }
    return ctx;
}
//# sourceMappingURL=ui-state-context.js.map