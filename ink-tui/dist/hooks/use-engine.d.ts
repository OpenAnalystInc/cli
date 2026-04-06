/**
 * useEngine() — React hook that manages bidirectional JSON-line communication
 * with the Rust engine child process.
 *
 * Features:
 * - Spawns the Rust binary as a child process (path configurable)
 * - Reads stdout line-by-line, parses JSON, validates with Zod schemas
 * - Dispatches parsed events to registered handlers
 * - Provides typed action methods (sendPrompt, cancelAgent, etc.)
 * - Handles process crash with optional auto-restart
 * - Tracks connection state: connecting | connected | disconnected | error
 * - Mock mode for UI development without a real engine
 */
import { type EngineEvent, type StreamDelta, type StreamEnd, type ToolCallStart, type ToolCallUpdate, type ToolCallEnd, type PermissionRequest, type AskUserRequest, type StatusUpdate, type AgentSpawned, type AgentStatusChanged, type AgentCompleted, type AgentFailed, type UsageUpdate, type KbResult, type SystemMessage, type Banner, type SidebarUpdate, type ModelInfo, type ContextFilesUpdate, type PermissionMode, type ActionCategory, type TuiAction } from '../types/messages.js';
import { type ConnectionState } from '../types/protocol.js';
export interface EngineEventHandlers {
    onStreamDelta?: (event: StreamDelta) => void;
    onStreamEnd?: (event: StreamEnd) => void;
    onToolCallStart?: (event: ToolCallStart) => void;
    onToolCallUpdate?: (event: ToolCallUpdate) => void;
    onToolCallComplete?: (event: ToolCallEnd) => void;
    onPermissionRequest?: (event: PermissionRequest) => void;
    onAskUserRequest?: (event: AskUserRequest) => void;
    onStatusUpdate?: (event: StatusUpdate) => void;
    onAgentSpawned?: (event: AgentSpawned) => void;
    onAgentStatusChanged?: (event: AgentStatusChanged) => void;
    onAgentCompleted?: (event: AgentCompleted) => void;
    onAgentFailed?: (event: AgentFailed) => void;
    onUsageUpdate?: (event: UsageUpdate) => void;
    onKbResult?: (event: KbResult) => void;
    onSystemMessage?: (event: SystemMessage) => void;
    onBanner?: (event: Banner) => void;
    onSidebarUpdate?: (event: SidebarUpdate) => void;
    onModelInfo?: (event: ModelInfo) => void;
    onContextFilesUpdate?: (event: ContextFilesUpdate) => void;
    onConnectionStateChange?: (state: ConnectionState) => void;
    onParseError?: (line: string, error: unknown) => void;
}
export interface EngineConfig {
    /** Path to the Rust engine binary. Defaults to 'openanalyst'. */
    binaryPath?: string;
    /** Arguments to pass to the engine. */
    args?: string[];
    /** Working directory for the engine process. */
    cwd?: string;
    /** Environment variables to set on the engine process. */
    env?: Record<string, string>;
    /** Auto-restart on crash. Defaults to false. */
    autoRestart?: boolean;
    /** Max restart attempts before giving up. Defaults to 3. */
    maxRestartAttempts?: number;
    /** If true, use mock engine instead of a real process. */
    mock?: boolean;
}
export interface UseEngineReturn {
    /** Current connection state. */
    connectionState: ConnectionState;
    /** Send a user prompt to the engine. */
    sendPrompt: (text: string, opts?: {
        effortBudget?: number;
        modelOverride?: string;
    }) => void;
    /** Submit a prompt to run in the background. */
    runInBackground: (text: string) => void;
    /** Cancel a running agent (current agent if no ID specified). */
    cancelAgent: (agentId?: string) => void;
    /** Resolve a permission request. */
    resolvePermission: (requestId: string, decision: 'allow' | 'deny') => void;
    /** Resolve an ask-user dialog. */
    resolveAskUser: (requestId: string, answer: string) => void;
    /** Submit knowledge base feedback. */
    sendKbFeedback: (queryId: number, rating: 'positive' | 'negative' | 'corrected', comment?: string, correction?: string) => void;
    /** Change permission mode (Ctrl+P cycle). */
    changePermissionMode: (mode: PermissionMode) => void;
    /** Toggle a context file. */
    toggleContextFile: (path: string, action: 'add' | 'remove') => void;
    /** Change routing for an action category. */
    changeRouting: (category: ActionCategory, tier: 'fast' | 'balanced' | 'capable') => void;
    /** Clear chat (Ctrl+L). */
    clearChat: () => void;
    /** Send a slash command. */
    slashCommand: (command: string) => void;
    /** Change the default model. */
    updateModel: (model: string) => void;
    /** Dispatch parallel agent commands (MOE). */
    moeDispatch: (commands: string[]) => void;
    /** Inject a skill while agents are working. */
    injectSkill: (command: string) => void;
    /** Tell the engine to quit. */
    quit: () => void;
    /** Manually restart the engine process. */
    restart: () => void;
}
export declare function useEngine(config?: EngineConfig, handlers?: EngineEventHandlers): UseEngineReturn;
export interface MockEngine {
    /** Emit a mock event to registered handlers. */
    emit: (event: EngineEvent) => void;
    /** Register an event handler. Returns unsubscribe function. */
    on: (handler: (event: EngineEvent) => void) => () => void;
    /** Send an action (triggers mock responses). */
    send: (action: TuiAction) => void;
    /** Dispose of the mock engine. */
    dispose: () => void;
}
/**
 * Create a standalone mock engine instance for testing and UI development.
 * This is not a React hook — use it in test files or non-React contexts.
 */
export declare function createMockEngine(): MockEngine;
