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
import { type SessionMetadata } from '../utils/session-manager.js';
export interface UseSessionReturn {
    /** Current session ID. */
    sessionId: string;
    /** Manually trigger a save. */
    save(): void;
    /** Whether auto-save is running. */
    autoSaveEnabled: boolean;
    /** Epoch ms of the last successful save, or null if never saved. */
    lastSavedAt: number | null;
    /** Load a specific session by ID. Returns true if successful. */
    loadSession(id: string): boolean;
    /** Load the most recent session. Returns true if successful. */
    resumeLatest(): boolean;
    /** List all available sessions (metadata only). */
    listSessions(): SessionMetadata[];
}
export declare function useSession(): UseSessionReturn;
