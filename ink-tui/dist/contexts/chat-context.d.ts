/**
 * ChatProvider — stores the chat message list and exposes mutation methods.
 *
 * This context is the single source of truth for all messages displayed
 * in the chat panel. Engine events (stream_delta, tool_call_start, etc.)
 * are translated into ChatMessage entries here.
 *
 * Split into two contexts for performance:
 *   - ChatMessagesContext — the messages array (changes on every delta)
 *   - ChatActionsContext — stable mutation functions (never change identity)
 */
import React, { type ReactNode } from 'react';
import type { ChatMessage, KBResultChatMessage, BannerChatMessage, FileOutputType } from '../types/chat.js';
import type { SystemLevel } from '../types/messages.js';
export interface ChatActions {
    /** Add a user message. */
    pushUser(text: string): void;
    /** Start or push a streaming delta for the current assistant message. */
    pushDelta(agentId: string, content: string): void;
    /** Mark the current assistant message as done streaming. */
    finishAssistant(agentId: string): void;
    /** Add a system message (info/warning/error). */
    pushSystem(text: string, level: SystemLevel): void;
    /** Add a tool call start. */
    pushToolCallStart(agentId: string, toolId: string, toolName: string, inputPreview: string): void;
    /** Update a tool call's output. */
    updateToolCall(toolId: string, output: string): void;
    /** Complete a tool call. */
    completeToolCall(toolId: string, status: 'completed' | 'failed', output: string, durationMs: number): void;
    /** Add a banner message. */
    pushBanner(data: Omit<BannerChatMessage, 'kind' | 'id' | 'timestamp'>): void;
    /** Add a KB result. */
    pushKBResult(data: Omit<KBResultChatMessage, 'kind' | 'id' | 'timestamp' | 'expanded' | 'activeTab'>): void;
    /** Add a file output message. */
    pushFileOutput(fileType: FileOutputType, description: string, filePath: string): void;
    /** Toggle the expanded state of a tool call card. */
    toggleToolCardExpand(toolId: string): void;
    /** Toggle the expanded state of a KB result card. */
    toggleKBExpand(id: string): void;
    /** Set the active tab on a KB result card. */
    setKBActiveTab(id: string, tab: number): void;
    /** Get a message by its ID. */
    getMessageById(id: string): ChatMessage | undefined;
    /** Bulk-load messages (used by session resume). Replaces current messages. */
    loadMessages(msgs: ChatMessage[]): void;
    /** Clear all messages. */
    clearAll(): void;
}
export interface ChatProviderProps {
    children: ReactNode;
}
export declare function ChatProvider({ children }: ChatProviderProps): React.ReactElement;
/** Returns the current messages array. Re-renders on every change. */
export declare function useChatMessages(): readonly ChatMessage[];
/** Returns stable mutation functions. Never causes re-renders. */
export declare function useChatActions(): ChatActions;
