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
import React, { type ReactNode } from 'react';
import { EngineBridge, type BridgeConfig } from './bridge.js';
import type { PermissionMode } from '../types/messages.js';
export interface EngineContextValue {
    /** The underlying bridge instance. */
    bridge: EngineBridge;
    /** Submit a user prompt. */
    submitPrompt: (text: string, opts?: {
        effortBudget?: number;
        modelOverride?: string;
    }) => void;
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
export interface EngineProviderProps {
    config: BridgeConfig;
    children: ReactNode;
}
export declare function EngineProvider({ config, children }: EngineProviderProps): React.ReactElement;
/**
 * Access the engine bridge and its convenience methods.
 * Must be used within an EngineProvider.
 */
export declare function useEngine(): EngineContextValue;
