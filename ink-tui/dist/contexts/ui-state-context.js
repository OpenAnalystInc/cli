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
import React, { createContext, useContext, useMemo, useReducer, } from 'react';
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
        sidebarAgents: [],
        sidebarFiles: [],
        sidebarPlans: [],
        sidebarRouting: {
            explore: { model: '', tier: '' },
            research: { model: '', tier: '' },
            code: { model: '', tier: '' },
            write: { model: '', tier: '' },
        },
        sidebarActivity: {
            backgroundTasks: 0,
            toolCallCount: 0,
            mcpServers: 0,
        },
        totalInputTokens: 0,
        totalOutputTokens: 0,
        modelCosts: {},
        terminalWidth: process.stdout.columns ?? 80,
        terminalHeight: process.stdout.rows ?? 24,
        exitPending: false,
        toastMessage: null,
        toastType: 'info',
    };
}
function uiReducer(state, action) {
    switch (action.type) {
        case 'TOGGLE_SIDEBAR': {
            if (state.sidebarVisible) {
                // Visible -> hide completely
                return { ...state, sidebarVisible: false, sidebarFocused: false, mode: 'idle' };
            }
            // Hidden -> show as focused popup
            return { ...state, sidebarVisible: true, sidebarFocused: true, mode: 'sidebar_focused' };
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
        case 'SET_SIDEBAR_AGENTS':
            return { ...state, sidebarAgents: action.agents };
        case 'SET_SIDEBAR_FILES':
            return { ...state, sidebarFiles: action.files };
        case 'SET_SIDEBAR_PLANS':
            return { ...state, sidebarPlans: action.plans };
        case 'SET_SIDEBAR_ROUTING':
            return { ...state, sidebarRouting: action.routing };
        case 'SET_SIDEBAR_ACTIVITY':
            return { ...state, sidebarActivity: action.activity };
        case 'UPDATE_AGENT_STATUS':
            return {
                ...state,
                sidebarAgents: state.sidebarAgents.map((a) => a.agentId === action.agentId ? { ...a, status: action.status } : a),
            };
        case 'ADD_USAGE': {
            const modelName = action.model || state.currentModel || 'unknown';
            const prev = state.modelCosts[modelName] || { inputTokens: 0, outputTokens: 0, cost: 0 };
            // Accurate per-model pricing ($ per 1M tokens) — April 2026 rates
            // Sources: openai.com/api/pricing, platform.claude.com/docs, ai.google.dev, docs.x.ai
            // [input_cost, output_cost] per 1M tokens
            const MODEL_PRICING = {
                // ── Anthropic (Claude) — platform.claude.com/docs/en/about-claude/pricing ──
                'Opus 4.6': [5.00, 25.00], // Latest: $5/$25 (67% cheaper than Opus 4)
                'Opus 4': [15.00, 75.00],
                'Sonnet 4.6': [3.00, 15.00], // Latest
                'Sonnet 4': [3.00, 15.00],
                'Haiku 4.5': [1.00, 5.00],
                'Haiku 4': [0.25, 1.25],
                // ── OpenAI — openai.com/api/pricing ──
                'GPT-5.4': [2.50, 15.00], // Latest: March 2026, 1M context
                'GPT-5.4 Pro': [30.00, 180.00], // Premium variant
                'GPT-5.4 Mini': [0.40, 1.60],
                'GPT-5.4 Nano': [0.10, 0.40],
                'GPT-5': [5.00, 15.00],
                'GPT-4o': [2.50, 10.00],
                'GPT-4o Mini': [0.15, 0.60],
                'GPT-4.1': [2.00, 8.00],
                'GPT-4.1 Mini': [0.40, 1.60],
                'GPT-4.1 Nano': [0.10, 0.40],
                'o3': [10.00, 40.00],
                'o3 Mini': [1.10, 4.40],
                'o3 Pro': [20.00, 80.00],
                'o4 Mini': [1.10, 4.40],
                'Codex Mini': [1.50, 6.00],
                // ── Google Gemini — ai.google.dev/gemini-api/docs/pricing ──
                'Gemini 3.1 Pro': [2.00, 12.00], // Latest generation
                'Gemini 3.1 Flash-Lite': [0.15, 0.60],
                'Gemini 3 Flash': [0.50, 3.00],
                'Gemini 2.5 Pro': [1.25, 10.00],
                'Gemini 2.5 Flash': [0.30, 2.50], // Updated pricing
                'Gemini 2.5 Flash-Lite': [0.10, 0.40],
                'Gemini 2.0 Flash': [0.10, 0.40], // Deprecated June 2026
                'Gemini 1.5 Pro': [1.25, 5.00],
                'Gemini 1.5 Flash': [0.075, 0.30],
                // ── xAI (Grok) — docs.x.ai/developers/models ──
                'Grok 4': [5.00, 25.00], // Latest
                'Grok 4 Fast': [0.20, 0.50],
                'Grok 4.1 Fast': [0.20, 0.50],
                'Grok Code Fast': [0.20, 0.50],
                'Grok 3': [3.00, 15.00],
                'Grok 3 Mini': [0.30, 0.50],
                'Grok 2': [2.00, 10.00],
                // ── DeepSeek — deepseek.com ──
                'DeepSeek V4': [0.30, 0.50], // Latest: March 2026, 1T params
                'DeepSeek R2': [0.55, 2.19], // Reasoning model
                'DeepSeek V3': [0.27, 1.10],
                'DeepSeek R1': [0.55, 2.19],
                // ── Meta Llama (via OpenRouter/Bedrock) ──
                'Llama 4 Maverick': [0.20, 0.60],
                'Llama 4 Scout': [0.15, 0.40],
                // ── Mistral ──
                'Mistral Large': [2.00, 6.00],
                'Codestral': [0.30, 0.90],
                // ── OpenRouter ──
                'Auto (best available)': [3.00, 15.00],
                // ── OpenAnalyst ──
                'OpenAnalyst Beta': [3.00, 15.00],
            };
            const pricing = MODEL_PRICING[modelName];
            const inputRate = pricing ? pricing[0] : 3.0; // fallback balanced
            const outputRate = pricing ? pricing[1] : 15.0;
            const addedCost = (action.inputTokens / 1_000_000) * inputRate
                + (action.outputTokens / 1_000_000) * outputRate;
            return {
                ...state,
                totalInputTokens: state.totalInputTokens + action.inputTokens,
                totalOutputTokens: state.totalOutputTokens + action.outputTokens,
                modelCosts: {
                    ...state.modelCosts,
                    [modelName]: {
                        inputTokens: prev.inputTokens + action.inputTokens,
                        outputTokens: prev.outputTokens + action.outputTokens,
                        cost: prev.cost + addedCost,
                    },
                },
            };
        }
        case 'CLEAR_CHAT':
            return {
                ...state,
                scrollOffset: 0,
                focusedMessageIndex: -1,
                autoScroll: true,
                scrollMode: false,
                mode: 'idle',
            };
        case 'SHOW_TOAST':
            return { ...state, toastMessage: action.message, toastType: action.toastType };
        case 'DISMISS_TOAST':
            return { ...state, toastMessage: null };
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
    // Toast auto-dismiss timer ref
    const toastTimerRef = React.useRef(null);
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
        setSidebarAgents: (agents) => dispatch({ type: 'SET_SIDEBAR_AGENTS', agents }),
        setSidebarFiles: (files) => dispatch({ type: 'SET_SIDEBAR_FILES', files }),
        setSidebarPlans: (plans) => dispatch({ type: 'SET_SIDEBAR_PLANS', plans }),
        setSidebarRouting: (routing) => dispatch({ type: 'SET_SIDEBAR_ROUTING', routing }),
        setSidebarActivity: (activity) => dispatch({ type: 'SET_SIDEBAR_ACTIVITY', activity }),
        updateAgentStatus: (agentId, status) => dispatch({ type: 'UPDATE_AGENT_STATUS', agentId, status }),
        addUsage: (inputTokens, outputTokens, model) => dispatch({ type: 'ADD_USAGE', inputTokens, outputTokens, model }),
        clearChat: () => dispatch({ type: 'CLEAR_CHAT' }),
        showToast: (message, durationMs = 2000, type = 'info') => {
            if (toastTimerRef.current)
                clearTimeout(toastTimerRef.current);
            dispatch({ type: 'SHOW_TOAST', message, toastType: type });
            toastTimerRef.current = setTimeout(() => {
                dispatch({ type: 'DISMISS_TOAST' });
                toastTimerRef.current = null;
            }, durationMs);
        },
        dismissToast: () => {
            if (toastTimerRef.current)
                clearTimeout(toastTimerRef.current);
            dispatch({ type: 'DISMISS_TOAST' });
        },
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