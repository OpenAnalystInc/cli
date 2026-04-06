import { jsx as _jsx } from "react/jsx-runtime";
/**
 * EngineProvider — React context that connects the EngineBridge to the UI.
 *
 * This is the integration glue. It:
 *   1. Creates an EngineBridge on mount (real or mock based on config)
 *   2. Listens to bridge 'event' emissions
 *   3. Maps each EngineEvent to the appropriate ChatActions and UIActions calls
 *   4. Exposes the bridge and convenience methods via useEngine()
 *
 * Provider order in the tree:
 *   UIStateProvider > ChatProvider > EngineProvider > layout
 *
 * This means EngineProvider can safely call useChatActions() and useUIActions().
 */
import { createContext, useContext, useEffect, useRef, useMemo, } from 'react';
import { EngineBridge } from './bridge.js';
import { useChatActions } from '../contexts/chat-context.js';
import { useUIActions } from '../contexts/ui-state-context.js';
import { providerPreferences } from '../utils/provider-preferences.js';
const EngineContext = createContext(null);
export function EngineProvider({ config, children }) {
    const chat = useChatActions();
    const ui = useUIActions();
    // Create bridge once, keep in ref
    const bridgeRef = useRef(null);
    if (bridgeRef.current === null) {
        bridgeRef.current = new EngineBridge(config);
    }
    const bridge = bridgeRef.current;
    // Wire bridge events to chat/UI on mount
    useEffect(() => {
        const handleEvent = (event) => {
            switch (event.type) {
                // -- Streaming --
                case 'stream_delta':
                    if (!event.done) {
                        chat.pushDelta(event.agentId, event.content);
                    }
                    break;
                case 'stream_end':
                    chat.finishAssistant(event.agentId);
                    break;
                // -- Tool calls --
                case 'tool_call_start':
                    chat.pushToolCallStart(event.agentId, event.toolId, event.toolName, event.inputPreview);
                    break;
                case 'tool_call_update':
                    chat.updateToolCall(event.toolId, event.output);
                    break;
                case 'tool_call_complete':
                    chat.completeToolCall(event.toolId, event.status, event.output, event.durationMs);
                    break;
                // -- Dialogs --
                case 'permission_request':
                    ui.showPermissionDialog({
                        requestId: event.requestId,
                        agentId: event.agentId,
                        toolName: event.toolName,
                        toolInput: event.toolInput,
                        requiredMode: event.requiredMode,
                        filePath: event.filePath,
                        description: event.description,
                        selectedButton: 'allow',
                    });
                    break;
                case 'ask_user_request':
                    ui.showAskUserDialog({
                        requestId: event.requestId,
                        agentId: event.agentId,
                        question: event.question,
                        options: event.options,
                        defaultValue: event.defaultValue,
                        allowFreeText: event.allowFreeText,
                        selectedIndex: 0,
                        typingMode: false,
                        typedText: event.defaultValue ?? '',
                    });
                    break;
                // -- Status --
                case 'status_update':
                    ui.setPhase(event.phase, event.label);
                    ui.setElapsed(event.elapsedMs);
                    if (event.tokensRemaining != null) {
                        ui.setTokensRemaining(event.tokensRemaining);
                    }
                    // Update input mode based on phase
                    if (event.phase === 'idle' || event.phase === 'done' || event.phase === 'error') {
                        ui.setInputMode('ready');
                    }
                    else {
                        ui.setInputMode('agent_running', event.label);
                    }
                    break;
                // -- Agent lifecycle --
                case 'agent_spawned':
                    ui.setActiveAgent(event.agentId);
                    chat.pushSystem(`Agent spawned: ${event.agentType} -- ${event.task}`, 'info');
                    break;
                case 'agent_status_changed':
                    // Could update a sidebar agent list -- for now, system message
                    break;
                case 'agent_completed':
                    ui.setActiveAgent(null);
                    break;
                case 'agent_failed':
                    ui.setActiveAgent(null);
                    chat.pushSystem(`Agent failed: ${event.error}`, 'error');
                    break;
                // -- Usage --
                case 'usage_update':
                    // Could display token counts in status bar or sidebar
                    break;
                // -- KB results --
                case 'kb_result':
                    chat.pushKBResult({
                        queryId: event.queryId,
                        query: event.query,
                        intent: event.intent,
                        answer: event.answer,
                        latencyMs: event.latencyMs,
                        fromCache: event.fromCache,
                    });
                    break;
                // -- System messages --
                case 'system_message':
                    chat.pushSystem(event.content, event.level);
                    break;
                // -- Banner --
                case 'banner':
                    chat.pushBanner({
                        version: event.version,
                        displayName: event.displayName,
                        email: event.email,
                        organization: event.organization,
                        provider: event.provider,
                        modelDisplay: event.modelDisplay,
                        workingDir: event.workingDir,
                        credits: event.credits,
                        tips: event.tips,
                    });
                    ui.setModelInfo(event.modelDisplay);
                    break;
                // -- Sidebar data --
                case 'sidebar_update':
                    // Sidebar data handled by sidebar component reading from a store,
                    // but we update a few UI state fields from it
                    if (event.activity.creditBalance) {
                        ui.setCreditBalance(event.activity.creditBalance);
                    }
                    ui.setMcpServerCount(event.activity.mcpServers);
                    break;
                // -- Model info --
                case 'model_info':
                    ui.setModelInfo(`${event.name} (${event.provider})`);
                    break;
                // -- Context files --
                case 'context_files_update':
                    for (const file of event.files) {
                        if (file.action === 'added') {
                            ui.addContextFile(file.path);
                        }
                        else {
                            ui.removeContextFile(file.path);
                        }
                    }
                    break;
            }
        };
        const handleConnectionState = (state) => {
            if (state === 'error') {
                chat.pushSystem('Engine connection lost', 'error');
                ui.setPhase('error');
                ui.setInputMode('ready');
            }
            else if (state === 'connected') {
                // Connection established -- engine will send banner.
                // Send the user's default provider/model to the engine so it uses
                // the correct provider for the first prompt.
                const defaultProvider = providerPreferences.getDefaultProvider();
                if (defaultProvider) {
                    const defaultModel = providerPreferences.getDefaultModelForProvider(defaultProvider);
                    if (defaultModel) {
                        bridge.updateModel(defaultModel.id);
                    }
                }
            }
        };
        bridge.on('event', handleEvent);
        bridge.on('connection_state', handleConnectionState);
        // Start the bridge
        bridge.start();
        return () => {
            bridge.removeListener('event', handleEvent);
            bridge.removeListener('connection_state', handleConnectionState);
            bridge.stop();
        };
    }, [bridge, chat, ui]);
    // Build stable context value
    const value = useMemo(() => ({
        bridge,
        submitPrompt(text, opts) {
            // Add user message to chat immediately
            chat.pushUser(text);
            bridge.submitPrompt(text, opts);
        },
        cancelAgent(agentId) {
            bridge.cancelAgent(agentId);
        },
        resolvePermission(requestId, decision) {
            bridge.resolvePermission(requestId, decision);
            ui.dismissPermissionDialog();
        },
        resolveAskUser(requestId, answer) {
            bridge.resolveAskUser(requestId, answer);
            ui.dismissAskUserDialog();
        },
        sendKbFeedback(queryId, rating, comment, correction) {
            bridge.sendKbFeedback(queryId, rating, comment, correction);
        },
        changePermissionMode(mode) {
            bridge.changePermissionMode(mode);
        },
        clearChat() {
            bridge.clearChat();
            chat.clearAll();
            ui.clearChat();
        },
        quit() {
            bridge.quit();
            bridge.stop();
        },
        restart() {
            bridge.restart();
            chat.clearAll();
            ui.clearChat();
        },
    }), [bridge, chat, ui]);
    return (_jsx(EngineContext.Provider, { value: value, children: children }));
}
// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------
/**
 * Access the engine bridge and its convenience methods.
 * Must be used within an EngineProvider.
 */
export function useEngine() {
    const ctx = useContext(EngineContext);
    if (!ctx) {
        throw new Error('useEngine() must be used within an <EngineProvider>');
    }
    return ctx;
}
//# sourceMappingURL=engine-context.js.map