/**
 * MessageList — renders the chat message array, dispatching to the
 * correct component for each message type.
 *
 * Message types:
 *  - user       -> UserMessage
 *  - assistant  -> AssistantMessage
 *  - system     -> SystemMessage
 *  - tool_call  -> ToolCard
 *  - kb_result  -> KnowledgeCard
 *  - banner     -> Banner
 */
import React from 'react';
import type { ChatMessage } from '../types/chat.js';
export interface MessageListProps {
    messages: readonly ChatMessage[];
    focusedIndex: number;
}
export declare const MessageList: React.MemoExoticComponent<({ messages, focusedIndex, }: MessageListProps) => React.ReactElement>;
