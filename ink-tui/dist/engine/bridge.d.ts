/**
 * EngineBridge — class-based wrapper around the child-process management logic.
 *
 * This is NOT a React component. It manages the Rust engine child process lifecycle
 * (or a mock simulator) and emits typed events via EventEmitter. The EngineProvider
 * context uses this class internally, but it can also be used standalone in tests
 * or non-React scripts.
 *
 * Communication protocol:
 *   - stdin:  TUI -> Engine  (JSON Lines, one action per line)
 *   - stdout: Engine -> TUI  (JSON Lines, one event per line)
 *   - stderr: Engine logs (forwarded as system_message with level: error)
 */
import { EventEmitter } from 'node:events';
import { type EngineEvent, type PermissionMode, type ActionCategory, type TuiAction } from '../types/messages.js';
import { type ConnectionState } from '../types/protocol.js';
export interface BridgeConfig {
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
export interface BridgeEvents {
    event: [EngineEvent];
    connection_state: [ConnectionState];
    parse_error: [line: string, error: unknown];
    exit: [code: number | null, signal: string | null];
}
export declare class EngineBridge extends EventEmitter {
    private proc;
    private rl;
    private mockEmitter;
    private restartCount;
    private _connectionState;
    private _config;
    private disposed;
    constructor(config?: BridgeConfig);
    get isConnected(): boolean;
    get connectionState(): ConnectionState;
    start(): void;
    stop(): void;
    restart(): void;
    dispose(): void;
    /** Send a raw JSON line to the engine. */
    send(jsonLine: string): void;
    /** Send a typed action. */
    sendAction<T extends TuiAction['type']>(type: T, payload: Omit<Extract<TuiAction, {
        type: T;
    }>, 'type' | 'timestamp'>): void;
    submitPrompt(text: string, opts?: {
        effortBudget?: number;
        modelOverride?: string;
    }): void;
    cancelAgent(agentId?: string): void;
    resolvePermission(requestId: string, decision: 'allow' | 'deny'): void;
    resolveAskUser(requestId: string, answer: string): void;
    sendKbFeedback(queryId: number, rating: 'positive' | 'negative' | 'corrected', comment?: string, correction?: string): void;
    changePermissionMode(mode: PermissionMode): void;
    toggleContextFile(path: string, action: 'add' | 'remove'): void;
    changeRouting(category: ActionCategory, tier: 'fast' | 'balanced' | 'capable'): void;
    clearChat(): void;
    slashCommand(command: string): void;
    updateModel(model: string): void;
    moeDispatch(commands: string[]): void;
    injectSkill(command: string): void;
    quit(): void;
    private setConnectionState;
    private processLine;
    private spawnEngine;
    private startMock;
    private handleMockAction;
    private simulateMockResponse;
}
