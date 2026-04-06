/**
 * useSession — React hook for auto-saving and resuming chat sessions.
 *
 * Features:
 *   - Creates a unique session ID on mount
 *   - Auto-saves every 60 seconds
 *   - Exposes manual save, load, and resume functions
 *   - Reads messages from ChatMessagesContext
 *   - Reads UI state (contextFiles, permissionMode, currentModel) from UIStateContext
 *   - On resume: sanitizes stale messages (running tools -> failed, streaming -> done)
 */
import { useCallback, useEffect, useRef, useState } from 'react';
import { useChatMessages, useChatActions } from '../contexts/chat-context.js';
import { useUIState } from '../contexts/ui-state-context.js';
import { SessionManager, } from '../utils/session-manager.js';
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
/** Auto-save interval in milliseconds. */
const AUTO_SAVE_INTERVAL_MS = 60_000;
// ---------------------------------------------------------------------------
// Singleton manager (shared across hook instances)
// ---------------------------------------------------------------------------
let _manager = null;
function getManager() {
    if (!_manager) {
        _manager = new SessionManager();
    }
    return _manager;
}
// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------
export function useSession() {
    const messages = useChatMessages();
    const chatActions = useChatActions();
    const ui = useUIState();
    // Session ID — stable for the lifetime of this component mount
    const [sessionId] = useState(() => SessionManager.newId());
    const [lastSavedAt, setLastSavedAt] = useState(null);
    // Refs to avoid stale closures in the interval callback
    const messagesRef = useRef(messages);
    messagesRef.current = messages;
    const uiRef = useRef(ui);
    uiRef.current = ui;
    const sessionIdRef = useRef(sessionId);
    sessionIdRef.current = sessionId;
    const manager = getManager();
    // ── Build session data snapshot ──────────────────────────────────────
    const buildSessionData = useCallback(() => {
        const msgs = messagesRef.current;
        const uiState = uiRef.current;
        // Derive summary from first user message
        const firstUser = msgs.find((m) => m.kind === 'user');
        const summary = firstUser && 'text' in firstUser
            ? firstUser.text.slice(0, 120)
            : 'Empty session';
        return {
            metadata: {
                id: sessionIdRef.current,
                createdAt: msgs.length > 0 ? msgs[0].timestamp : Date.now(),
                updatedAt: Date.now(),
                messageCount: msgs.length,
                workingDir: process.cwd(),
                modelUsed: uiState.currentModel || 'unknown',
                summary,
            },
            messages: [...msgs],
            contextFiles: [...uiState.contextFiles],
            permissionMode: uiState.permissionMode,
        };
    }, []);
    // ── Save ─────────────────────────────────────────────────────────────
    const save = useCallback(() => {
        const data = buildSessionData();
        // Only save if there are meaningful messages (skip empty sessions)
        if (data.messages.length === 0)
            return;
        manager.save(data);
        setLastSavedAt(Date.now());
    }, [buildSessionData, manager]);
    // ── Auto-save interval ───────────────────────────────────────────────
    useEffect(() => {
        const timer = setInterval(() => {
            // Only auto-save if there are messages
            if (messagesRef.current.length > 0) {
                save();
            }
        }, AUTO_SAVE_INTERVAL_MS);
        return () => clearInterval(timer);
    }, [save]);
    // ── Load / Resume ────────────────────────────────────────────────────
    const loadSession = useCallback((id) => {
        const data = manager.load(id);
        if (!data)
            return false;
        const sanitized = SessionManager.sanitizeForResume(data.messages);
        chatActions.loadMessages(sanitized);
        return true;
    }, [manager, chatActions]);
    const resumeLatest = useCallback(() => {
        const data = manager.getLatest();
        if (!data)
            return false;
        const sanitized = SessionManager.sanitizeForResume(data.messages);
        chatActions.loadMessages(sanitized);
        return true;
    }, [manager, chatActions]);
    const listSessions = useCallback(() => {
        return manager.listSessions();
    }, [manager]);
    return {
        sessionId,
        save,
        autoSaveEnabled: true,
        lastSavedAt,
        loadSession,
        resumeLatest,
        listSessions,
    };
}
//# sourceMappingURL=use-session.js.map