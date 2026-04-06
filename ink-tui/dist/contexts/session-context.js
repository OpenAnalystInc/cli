import { jsx as _jsx } from "react/jsx-runtime";
/**
 * SessionProvider — wraps the useSession hook in a context so that any
 * component in the tree (including the input box for /resume) can access
 * session persistence without prop drilling.
 *
 * Provider order in the tree (placed inside EngineProvider so it can read
 * chat messages and UI state):
 *
 *   ... > ChatProvider > EngineProvider > SessionProvider > DefaultLayout
 *
 * Auto-save triggers:
 *   - Every 60 s (handled inside useSession)
 *   - After each assistant message finishes (stream_end)
 *   - After each tool call completes
 *   - On unmount (cleanup / graceful exit)
 */
import { createContext, useContext, useEffect, useRef, } from 'react';
import { useSession } from '../hooks/use-session.js';
import { useChatMessages } from './chat-context.js';
// ---------------------------------------------------------------------------
// Context
// ---------------------------------------------------------------------------
const SessionContext = createContext(null);
export function SessionProvider({ children }) {
    const session = useSession();
    const messages = useChatMessages();
    // Track message count to detect new completions
    const prevCountRef = useRef(messages.length);
    const prevLastKindRef = useRef(null);
    // Save whenever an assistant message finishes or a tool call completes
    useEffect(() => {
        const count = messages.length;
        const last = messages[count - 1];
        const prevCount = prevCountRef.current;
        const prevLastKind = prevLastKindRef.current;
        prevCountRef.current = count;
        prevLastKindRef.current = last?.kind ?? null;
        if (count <= prevCount || !last)
            return;
        // Save after assistant message finishes streaming
        if (last.kind === 'assistant' &&
            !last.streaming &&
            prevLastKind === 'assistant') {
            session.save();
            return;
        }
        // Save after tool call completes
        if (last.kind === 'tool_call' &&
            (last.status === 'completed' || last.status === 'failed')) {
            session.save();
            return;
        }
        // Save after user submits a prompt (so the user message is captured)
        if (last.kind === 'user') {
            session.save();
        }
    }, [messages, session]);
    // Save on unmount (graceful exit)
    const sessionRef = useRef(session);
    sessionRef.current = session;
    useEffect(() => {
        const onExit = () => {
            sessionRef.current.save();
        };
        process.on('exit', onExit);
        process.on('SIGINT', onExit);
        process.on('SIGTERM', onExit);
        return () => {
            // Final save on provider unmount
            sessionRef.current.save();
            process.removeListener('exit', onExit);
            process.removeListener('SIGINT', onExit);
            process.removeListener('SIGTERM', onExit);
        };
    }, []);
    return (_jsx(SessionContext.Provider, { value: session, children: children }));
}
// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------
/**
 * Access the session manager. Must be used within a <SessionProvider>.
 */
export function useSessionContext() {
    const ctx = useContext(SessionContext);
    if (!ctx) {
        throw new Error('useSessionContext() must be used within a <SessionProvider>');
    }
    return ctx;
}
//# sourceMappingURL=session-context.js.map