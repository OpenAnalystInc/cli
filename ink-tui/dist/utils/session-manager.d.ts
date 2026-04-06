/**
 * SessionManager — persists and restores chat sessions to/from disk.
 *
 * Sessions are stored as JSON files in `.openanalyst/sessions/` under the
 * project directory (or the user home as fallback).  Each file is named
 * `{sessionId}.json` and contains the full message history plus metadata.
 *
 * Design decisions:
 *   - Project-local sessions preferred (cwd/.openanalyst/sessions/)
 *   - Global fallback: ~/.openanalyst/sessions/
 *   - Unique IDs via timestamp + random suffix (no uuid dependency)
 *   - Large sessions capped at 500 messages on save to stay under disk budget
 *   - Corrupted files are skipped silently on list/load (logged to stderr)
 *   - No file locking — unique session IDs prevent collisions across instances
 */
import type { ChatMessage } from '../types/chat.js';
export interface SessionMetadata {
    /** Unique session identifier, e.g. `session-1712400000000-abc123`. */
    id: string;
    /** Epoch ms when the session was first created. */
    createdAt: number;
    /** Epoch ms of the most recent save. */
    updatedAt: number;
    /** Number of messages in the saved session. */
    messageCount: number;
    /** Absolute path of the working directory when the session was created. */
    workingDir: string;
    /** Display string for the model in use, e.g. "opus-4 (anthropic)". */
    modelUsed: string;
    /** One-liner summary — the first user message or auto-generated. */
    summary: string;
}
export interface SessionData {
    metadata: SessionMetadata;
    messages: ChatMessage[];
    contextFiles: string[];
    permissionMode: string;
}
export declare class SessionManager {
    /** Primary sessions dir (project-level preferred). */
    private readonly sessionsDir;
    /** Global sessions dir — always ~/.openanalyst/sessions/ as backup. */
    private readonly globalSessionsDir;
    constructor();
    /**
     * Save session to BOTH project-level AND global.
     * - Project copy: shared when project is pushed via git (team collaboration)
     * - Global copy: user's personal backup across all projects
     */
    save(data: SessionData): void;
    /** Load a session by its full ID. Returns null if not found or corrupted. */
    load(sessionId: string): SessionData | null;
    /** Load the most recently updated session. */
    getLatest(): SessionData | null;
    /** List all sessions, metadata only, sorted newest-first. */
    listSessions(): SessionMetadata[];
    /** Delete a session by ID. */
    delete(sessionId: string): boolean;
    /** Generate a new unique session ID. */
    static newId(): string;
    /**
     * Sanitize loaded messages for safe replay:
     * - Streaming assistant messages get `streaming: false`
     * - Running tool calls get `status: 'failed'`
     */
    static sanitizeForResume(messages: ChatMessage[]): ChatMessage[];
    /**
     * Resolve sessions directory.
     * Prefer project-local: cwd/.openanalyst/sessions/
     * Fallback to global: ~/.openanalyst/sessions/
     */
    private resolveSessionsDir;
    /** Ensure both session directories exist. */
    private ensureDir;
    /** Read and parse a single session file. Returns null on any error. */
    private readSessionFile;
}
