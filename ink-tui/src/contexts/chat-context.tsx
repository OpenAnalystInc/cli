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

import React, {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import type {
  ChatMessage,
  UserChatMessage,
  AssistantChatMessage,
  SystemChatMessage,
  ToolCallChatMessage,
  KBResultChatMessage,
  BannerChatMessage,
  FileOutputChatMessage,
  FileOutputType,
} from '../types/chat.js';
import { nextMessageId } from '../types/chat.js';
import type { SystemLevel } from '../types/messages.js';

// ---------------------------------------------------------------------------
// Actions interface
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Contexts
// ---------------------------------------------------------------------------

const ChatMessagesContext = createContext<readonly ChatMessage[] | null>(null);
const ChatActionsContext = createContext<ChatActions | null>(null);

// ---------------------------------------------------------------------------
// Provider
// ---------------------------------------------------------------------------

export interface ChatProviderProps {
  children: ReactNode;
}

export function ChatProvider({ children }: ChatProviderProps): React.ReactElement {
  const [messages, setMessages] = useState<ChatMessage[]>([]);

  // Ref for quick lookup of the current messages in callbacks
  const messagesRef = useRef(messages);
  messagesRef.current = messages;

  const actions = useMemo<ChatActions>(() => ({
    pushUser(text: string) {
      const isSlashCommand = text.trimStart().startsWith('/');
      const msg: UserChatMessage = {
        kind: 'user',
        id: nextMessageId(),
        text,
        isSlashCommand,
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, msg]);
    },

    pushDelta(agentId: string, content: string) {
      setMessages((prev) => {
        const last = prev[prev.length - 1];
        if (last && last.kind === 'assistant' && last.agentId === agentId && last.streaming) {
          // Append to existing streaming message
          const updated: AssistantChatMessage = {
            ...last,
            content: last.content + content,
          };
          return [...prev.slice(0, -1), updated];
        }
        // Create new assistant message
        const msg: AssistantChatMessage = {
          kind: 'assistant',
          id: nextMessageId(),
          agentId,
          content,
          streaming: true,
          timestamp: Date.now(),
        };
        return [...prev, msg];
      });
    },

    finishAssistant(agentId: string) {
      setMessages((prev) => {
        const last = prev[prev.length - 1];
        if (last && last.kind === 'assistant' && last.agentId === agentId) {
          const updated: AssistantChatMessage = {
            ...last,
            streaming: false,
          };
          return [...prev.slice(0, -1), updated];
        }
        return prev;
      });
    },

    pushSystem(text: string, level: SystemLevel) {
      setMessages((prev) => {
        // Prevent duplicate consecutive system messages
        const last = prev[prev.length - 1];
        if (last && last.kind === 'system' && last.text === text) {
          return prev;
        }
        const msg: SystemChatMessage = {
          kind: 'system',
          id: nextMessageId(),
          text,
          level,
          timestamp: Date.now(),
        };
        return [...prev, msg];
      });
    },

    pushToolCallStart(agentId: string, toolId: string, toolName: string, inputPreview: string) {
      const msg: ToolCallChatMessage = {
        kind: 'tool_call',
        id: nextMessageId(),
        agentId,
        toolId,
        toolName,
        inputPreview,
        status: 'running',
        output: '',
        durationMs: 0,
        expanded: false,
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, msg]);
    },

    updateToolCall(toolId: string, output: string) {
      setMessages((prev) => {
        const idx = prev.findIndex(
          (m) => m.kind === 'tool_call' && m.toolId === toolId,
        );
        if (idx === -1) return prev;
        const msg = prev[idx] as ToolCallChatMessage;
        const updated: ToolCallChatMessage = { ...msg, output };
        return [...prev.slice(0, idx), updated, ...prev.slice(idx + 1)];
      });
    },

    completeToolCall(toolId: string, status: 'completed' | 'failed', output: string, durationMs: number) {
      setMessages((prev) => {
        const idx = prev.findIndex(
          (m) => m.kind === 'tool_call' && m.toolId === toolId,
        );
        if (idx === -1) return prev;
        const msg = prev[idx] as ToolCallChatMessage;
        const updated: ToolCallChatMessage = { ...msg, status, output, durationMs };
        return [...prev.slice(0, idx), updated, ...prev.slice(idx + 1)];
      });
    },

    pushBanner(data) {
      const msg: BannerChatMessage = {
        kind: 'banner',
        id: nextMessageId(),
        timestamp: Date.now(),
        ...data,
      };
      setMessages((prev) => [...prev, msg]);
    },

    pushKBResult(data) {
      const msg: KBResultChatMessage = {
        kind: 'kb_result',
        id: nextMessageId(),
        timestamp: Date.now(),
        expanded: false,
        activeTab: 0,
        ...data,
      };
      setMessages((prev) => [...prev, msg]);
    },

    pushFileOutput(fileType: FileOutputType, description: string, filePath: string) {
      const msg: FileOutputChatMessage = {
        kind: 'file_output',
        id: nextMessageId(),
        fileType,
        description,
        filePath,
        timestamp: Date.now(),
      };
      setMessages((prev) => [...prev, msg]);
    },

    toggleToolCardExpand(toolId: string) {
      setMessages((prev) => {
        const idx = prev.findIndex(
          (m) => m.kind === 'tool_call' && m.toolId === toolId,
        );
        if (idx === -1) return prev;
        const msg = prev[idx] as ToolCallChatMessage;
        const updated: ToolCallChatMessage = { ...msg, expanded: !msg.expanded };
        return [...prev.slice(0, idx), updated, ...prev.slice(idx + 1)];
      });
    },

    toggleKBExpand(id: string) {
      setMessages((prev) => {
        const idx = prev.findIndex((m) => m.kind === 'kb_result' && m.id === id);
        if (idx === -1) return prev;
        const msg = prev[idx] as KBResultChatMessage;
        const updated: KBResultChatMessage = { ...msg, expanded: !msg.expanded };
        return [...prev.slice(0, idx), updated, ...prev.slice(idx + 1)];
      });
    },

    setKBActiveTab(id: string, tab: number) {
      setMessages((prev) => {
        const idx = prev.findIndex((m) => m.kind === 'kb_result' && m.id === id);
        if (idx === -1) return prev;
        const msg = prev[idx] as KBResultChatMessage;
        const updated: KBResultChatMessage = { ...msg, activeTab: tab };
        return [...prev.slice(0, idx), updated, ...prev.slice(idx + 1)];
      });
    },

    getMessageById(id: string): ChatMessage | undefined {
      return messagesRef.current.find((m) => m.id === id);
    },

    loadMessages(msgs: ChatMessage[]) {
      setMessages(msgs);
    },

    clearAll() {
      setMessages([]);
    },
  }), []);

  return (
    <ChatMessagesContext.Provider value={messages}>
      <ChatActionsContext.Provider value={actions}>
        {children}
      </ChatActionsContext.Provider>
    </ChatMessagesContext.Provider>
  );
}

// ---------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------

/** Returns the current messages array. Re-renders on every change. */
export function useChatMessages(): readonly ChatMessage[] {
  const ctx = useContext(ChatMessagesContext);
  if (ctx === null) {
    throw new Error('useChatMessages() must be used within a <ChatProvider>');
  }
  return ctx;
}

/** Returns stable mutation functions. Never causes re-renders. */
export function useChatActions(): ChatActions {
  const ctx = useContext(ChatActionsContext);
  if (ctx === null) {
    throw new Error('useChatActions() must be used within a <ChatProvider>');
  }
  return ctx;
}
