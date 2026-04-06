import { jsx as _jsx } from "react/jsx-runtime";
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
import { createContext, useContext, useMemo, useRef, useState, } from 'react';
import { nextMessageId } from '../types/chat.js';
// ---------------------------------------------------------------------------
// Contexts
// ---------------------------------------------------------------------------
const ChatMessagesContext = createContext(null);
const ChatActionsContext = createContext(null);
export function ChatProvider({ children }) {
    const [messages, setMessages] = useState([]);
    // Ref for quick lookup of the current messages in callbacks
    const messagesRef = useRef(messages);
    messagesRef.current = messages;
    const actions = useMemo(() => ({
        pushUser(text) {
            const isSlashCommand = text.trimStart().startsWith('/');
            const msg = {
                kind: 'user',
                id: nextMessageId(),
                text,
                isSlashCommand,
                timestamp: Date.now(),
            };
            setMessages((prev) => [...prev, msg]);
        },
        pushDelta(agentId, content) {
            setMessages((prev) => {
                const last = prev[prev.length - 1];
                if (last && last.kind === 'assistant' && last.agentId === agentId && last.streaming) {
                    // Append to existing streaming message
                    const updated = {
                        ...last,
                        content: last.content + content,
                    };
                    return [...prev.slice(0, -1), updated];
                }
                // Create new assistant message
                const msg = {
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
        finishAssistant(agentId) {
            setMessages((prev) => {
                const last = prev[prev.length - 1];
                if (last && last.kind === 'assistant' && last.agentId === agentId) {
                    const updated = {
                        ...last,
                        streaming: false,
                    };
                    return [...prev.slice(0, -1), updated];
                }
                return prev;
            });
        },
        pushSystem(text, level) {
            setMessages((prev) => {
                // Prevent duplicate consecutive system messages
                const last = prev[prev.length - 1];
                if (last && last.kind === 'system' && last.text === text) {
                    return prev;
                }
                const msg = {
                    kind: 'system',
                    id: nextMessageId(),
                    text,
                    level,
                    timestamp: Date.now(),
                };
                return [...prev, msg];
            });
        },
        pushToolCallStart(agentId, toolId, toolName, inputPreview) {
            const msg = {
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
        updateToolCall(toolId, output) {
            setMessages((prev) => {
                const idx = prev.findIndex((m) => m.kind === 'tool_call' && m.toolId === toolId);
                if (idx === -1)
                    return prev;
                const msg = prev[idx];
                const updated = { ...msg, output };
                return [...prev.slice(0, idx), updated, ...prev.slice(idx + 1)];
            });
        },
        completeToolCall(toolId, status, output, durationMs, diff) {
            setMessages((prev) => {
                const idx = prev.findIndex((m) => m.kind === 'tool_call' && m.toolId === toolId);
                if (idx === -1)
                    return prev;
                const msg = prev[idx];
                const updated = { ...msg, status, output, durationMs, diff };
                return [...prev.slice(0, idx), updated, ...prev.slice(idx + 1)];
            });
        },
        pushBanner(data) {
            const msg = {
                kind: 'banner',
                id: nextMessageId(),
                timestamp: Date.now(),
                ...data,
            };
            setMessages((prev) => [...prev, msg]);
        },
        pushKBResult(data) {
            const msg = {
                kind: 'kb_result',
                id: nextMessageId(),
                timestamp: Date.now(),
                expanded: false,
                activeTab: 0,
                ...data,
            };
            setMessages((prev) => [...prev, msg]);
        },
        pushFileOutput(fileType, description, filePath) {
            const msg = {
                kind: 'file_output',
                id: nextMessageId(),
                fileType,
                description,
                filePath,
                timestamp: Date.now(),
            };
            setMessages((prev) => [...prev, msg]);
        },
        toggleToolCardExpand(toolId) {
            setMessages((prev) => {
                const idx = prev.findIndex((m) => m.kind === 'tool_call' && m.toolId === toolId);
                if (idx === -1)
                    return prev;
                const msg = prev[idx];
                const updated = { ...msg, expanded: !msg.expanded };
                return [...prev.slice(0, idx), updated, ...prev.slice(idx + 1)];
            });
        },
        toggleKBExpand(id) {
            setMessages((prev) => {
                const idx = prev.findIndex((m) => m.kind === 'kb_result' && m.id === id);
                if (idx === -1)
                    return prev;
                const msg = prev[idx];
                const updated = { ...msg, expanded: !msg.expanded };
                return [...prev.slice(0, idx), updated, ...prev.slice(idx + 1)];
            });
        },
        setKBActiveTab(id, tab) {
            setMessages((prev) => {
                const idx = prev.findIndex((m) => m.kind === 'kb_result' && m.id === id);
                if (idx === -1)
                    return prev;
                const msg = prev[idx];
                const updated = { ...msg, activeTab: tab };
                return [...prev.slice(0, idx), updated, ...prev.slice(idx + 1)];
            });
        },
        getMessageById(id) {
            return messagesRef.current.find((m) => m.id === id);
        },
        loadMessages(msgs) {
            setMessages(msgs);
        },
        clearAll() {
            setMessages([]);
        },
    }), []);
    return (_jsx(ChatMessagesContext.Provider, { value: messages, children: _jsx(ChatActionsContext.Provider, { value: actions, children: children }) }));
}
// ---------------------------------------------------------------------------
// Hooks
// ---------------------------------------------------------------------------
/** Returns the current messages array. Re-renders on every change. */
export function useChatMessages() {
    const ctx = useContext(ChatMessagesContext);
    if (ctx === null) {
        throw new Error('useChatMessages() must be used within a <ChatProvider>');
    }
    return ctx;
}
/** Returns stable mutation functions. Never causes re-renders. */
export function useChatActions() {
    const ctx = useContext(ChatActionsContext);
    if (ctx === null) {
        throw new Error('useChatActions() must be used within a <ChatProvider>');
    }
    return ctx;
}
//# sourceMappingURL=chat-context.js.map