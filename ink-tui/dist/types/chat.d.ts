/**
 * Chat message types for the message list.
 *
 * These are the *rendered* message types that the ChatPanel displays.
 * They are derived from engine events (StreamDelta, ToolCallStart, etc.)
 * but represent the final display state of each message.
 */
import type { DiffInfo, SubQuestionResult, SystemLevel } from './messages.js';
export interface UserChatMessage {
    readonly kind: 'user';
    readonly id: string;
    readonly text: string;
    readonly isSlashCommand: boolean;
    readonly timestamp: number;
}
export interface AssistantChatMessage {
    readonly kind: 'assistant';
    readonly id: string;
    readonly agentId: string;
    /** Accumulated markdown content. Grows during streaming. */
    content: string;
    /** True while the LLM is still generating tokens. */
    streaming: boolean;
    readonly timestamp: number;
}
export interface SystemChatMessage {
    readonly kind: 'system';
    readonly id: string;
    readonly text: string;
    readonly level: SystemLevel;
    readonly timestamp: number;
}
export interface ToolCallChatMessage {
    readonly kind: 'tool_call';
    readonly id: string;
    readonly agentId: string;
    readonly toolId: string;
    readonly toolName: string;
    readonly inputPreview: string;
    status: 'running' | 'completed' | 'failed';
    output: string;
    durationMs: number;
    diff?: DiffInfo;
    expanded: boolean;
    readonly timestamp: number;
}
export interface KBResultChatMessage {
    readonly kind: 'kb_result';
    readonly id: string;
    readonly queryId: number;
    readonly query: string;
    readonly intent: string;
    readonly subQuestions: readonly SubQuestionResult[];
    readonly answer?: string;
    readonly latencyMs: number;
    readonly fromCache: boolean;
    expanded: boolean;
    activeTab: number;
    readonly timestamp: number;
}
export interface BannerChatMessage {
    readonly kind: 'banner';
    readonly id: string;
    readonly version: string;
    readonly displayName: string;
    readonly email?: string;
    readonly organization?: string;
    readonly provider: string;
    readonly modelDisplay: string;
    readonly workingDir: string;
    readonly credits?: string;
    readonly tips: readonly string[];
    readonly timestamp: number;
}
export type FileOutputType = 'image' | 'audio' | 'diagram' | 'text';
export interface FileOutputChatMessage {
    readonly kind: 'file_output';
    readonly id: string;
    readonly fileType: FileOutputType;
    readonly description: string;
    readonly filePath: string;
    readonly timestamp: number;
}
export type ChatMessage = UserChatMessage | AssistantChatMessage | SystemChatMessage | ToolCallChatMessage | KBResultChatMessage | BannerChatMessage | FileOutputChatMessage;
export declare function nextMessageId(): string;
