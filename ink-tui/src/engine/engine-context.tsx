/**
 * EngineProvider — React context that connects the EngineBridge to the UI.
 *
 * This is the integration glue. It:
 *   1. Creates an EngineBridge on mount (connects to Rust engine)
 *   2. Listens to bridge 'event' emissions
 *   3. Maps each EngineEvent to the appropriate ChatActions and UIActions calls
 *   4. Exposes the bridge and convenience methods via useEngine()
 *
 * Provider order in the tree:
 *   UIStateProvider > ChatProvider > EngineProvider > layout
 *
 * This means EngineProvider can safely call useChatActions() and useUIActions().
 */

import React, {
  createContext,
  useContext,
  useEffect,
  useRef,
  useMemo,
  type ReactNode,
} from 'react';

import { EngineBridge, type BridgeConfig } from './bridge.js';
import { useChatActions } from '../contexts/chat-context.js';
import { useUIActions } from '../contexts/ui-state-context.js';
import type { EngineEvent, PermissionMode } from '../types/messages.js';
import type { ConnectionState } from '../types/protocol.js';
import { providerPreferences } from '../utils/provider-preferences.js';

// ---------------------------------------------------------------------------
// Context value
// ---------------------------------------------------------------------------

export interface EngineContextValue {
  /** The underlying bridge instance. */
  bridge: EngineBridge;

  /** Submit a user prompt. */
  submitPrompt: (text: string, opts?: { effortBudget?: number; modelOverride?: string }) => void;

  /** Cancel the current agent. */
  cancelAgent: (agentId?: string) => void;

  /** Resolve a permission dialog. */
  resolvePermission: (requestId: string, decision: 'allow' | 'deny') => void;

  /** Resolve an ask-user dialog. */
  resolveAskUser: (requestId: string, answer: string) => void;

  /** Send KB feedback. */
  sendKbFeedback: (queryId: number, rating: 'positive' | 'negative' | 'corrected', comment?: string, correction?: string) => void;

  /** Change permission mode. */
  changePermissionMode: (mode: PermissionMode) => void;

  /** Clear chat (engine + local). */
  clearChat: () => void;

  /** Tell engine to quit. */
  quit: () => void;

  /** Restart the engine. */
  restart: () => void;
}

const EngineContext = createContext<EngineContextValue | null>(null);

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

export interface EngineProviderProps {
  config: BridgeConfig;
  children: ReactNode;
}

export function EngineProvider({ config, children }: EngineProviderProps): React.ReactElement {
  const chat = useChatActions();
  const ui = useUIActions();

  // Create bridge once, keep in ref
  const bridgeRef = useRef<EngineBridge | null>(null);
  if (bridgeRef.current === null) {
    bridgeRef.current = new EngineBridge(config);
  }
  const bridge = bridgeRef.current;

  // Wire bridge events to chat/UI on mount
  useEffect(() => {
    const handleEvent = (event: EngineEvent) => {
      switch (event.type) {
        // -- Streaming --
        case 'stream_delta':
          // Rust sends "text" (not "content"), and no "done" field.
          // Stream completion is signaled by stream_end event.
          chat.pushDelta(event.agentId, event.text);
          break;

        case 'stream_end':
          chat.finishAssistant(event.agentId);
          break;

        // -- Tool calls --
        case 'tool_call_start':
          // Rust sends "callId" not "toolId"
          chat.pushToolCallStart(event.agentId, event.callId, event.toolName, event.inputPreview);
          break;

        case 'tool_call_update':
          // NOTE: This event does not exist in Rust yet. Kept for future use.
          chat.updateToolCall(event.callId, event.output);
          break;

        case 'tool_call_end':
          // Rust sends "tool_call_end" with "callId", "isError", "duration" (not "durationMs")
          chat.completeToolCall(
            event.callId,
            event.isError ? 'failed' : 'completed',
            event.output,
            event.duration,
            event.diff ?? undefined,
          );
          break;

        // -- Dialogs --
        case 'permission_request':
          // Rust sends "input" (not "toolInput"), "requiredMode" as free string.
          // No "filePath" or "description" from Rust.
          ui.showPermissionDialog({
            requestId: event.requestId,
            agentId: event.agentId,
            toolName: event.toolName,
            toolInput: event.input,
            requiredMode: event.requiredMode,
            selectedButton: 'allow',
          });
          break;

        case 'ask_user_request':
          // Rust sends "default" (not "defaultValue"), and no "allowFreeText" field.
          // Default allowFreeText to true when no options provided.
          ui.showAskUserDialog({
            requestId: event.requestId,
            agentId: event.agentId,
            question: event.question,
            options: event.options ?? undefined,
            defaultValue: event.default ?? undefined,
            allowFreeText: !event.options || event.options.length === 0,
            selectedIndex: 0,
            typingMode: false,
            typedText: event.default ?? '',
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
          } else {
            ui.setInputMode('agent_running', event.label);
            // Auto-hide sidebar when task starts (user can reopen with F2)
            ui.hideSidebar();
          }
          break;

        // -- Agent lifecycle --
        case 'agent_spawned':
          ui.setActiveAgent(event.agentId);
          chat.pushSystem(`Agent spawned: ${event.agentType} -- ${event.task}`, 'info');
          break;

        case 'agent_status_changed':
          ui.updateAgentStatus(event.agentId, event.status);
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
          ui.addUsage(event.inputTokens, event.outputTokens, (event as Record<string, unknown>).model as string | undefined);
          break;

        // -- KB results --
        case 'knowledge_result':
          chat.pushKBResult({
            queryId: event.queryId,
            query: event.query,
            intent: event.intent,
            subQuestions: event.subQuestions,
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
          ui.setSidebarAgents([...event.agents]);
          ui.setSidebarFiles([...event.files]);
          ui.setSidebarPlans([...event.plans]);
          ui.setSidebarRouting(event.routing);
          ui.setSidebarActivity(event.activity);
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
            } else {
              ui.removeContextFile(file.path);
            }
          }
          break;
      }
    };

    const handleConnectionState = (state: ConnectionState) => {
      if (state === 'error') {
        chat.pushSystem('Engine connection lost', 'error');
        ui.setPhase('error');
        ui.setInputMode('ready');
      } else if (state === 'connected') {
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
  const value = useMemo<EngineContextValue>(() => ({
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

  return (
    <EngineContext.Provider value={value}>
      {children}
    </EngineContext.Provider>
  );
}

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

/**
 * Access the engine bridge and its convenience methods.
 * Must be used within an EngineProvider.
 */
export function useEngine(): EngineContextValue {
  const ctx = useContext(EngineContext);
  if (!ctx) {
    throw new Error('useEngine() must be used within an <EngineProvider>');
  }
  return ctx;
}
