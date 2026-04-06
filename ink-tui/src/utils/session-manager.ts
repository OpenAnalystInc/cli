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

import * as fs from 'node:fs';
import * as path from 'node:path';
import * as os from 'node:os';
import type { ChatMessage, AssistantChatMessage, ToolCallChatMessage } from '../types/chat.js';

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/** Maximum messages retained on save. Prevents multi-MB session files. */
const MAX_MESSAGES_ON_SAVE = 500;

/** Directory name for session storage. */
const SESSION_DIR_NAME = '.openanalyst';
const SESSION_SUB_DIR = 'sessions';

// ---------------------------------------------------------------------------
// SessionManager
// ---------------------------------------------------------------------------

export class SessionManager {
  /** Primary sessions dir (project-level preferred). */
  private readonly sessionsDir: string;
  /** Global sessions dir — always ~/.openanalyst/sessions/ as backup. */
  private readonly globalSessionsDir: string;

  constructor() {
    this.globalSessionsDir = path.join(os.homedir(), SESSION_DIR_NAME, SESSION_SUB_DIR);
    this.sessionsDir = this.resolveSessionsDir();
    this.ensureDir();
  }

  // ── Public API ──────────────────────────────────────────────────────────

  /**
   * Save session to BOTH project-level AND global.
   * - Project copy: shared when project is pushed via git (team collaboration)
   * - Global copy: user's personal backup across all projects
   */
  save(data: SessionData): void {
    // Cap messages to prevent oversized files
    const trimmedMessages =
      data.messages.length > MAX_MESSAGES_ON_SAVE
        ? data.messages.slice(-MAX_MESSAGES_ON_SAVE)
        : data.messages;

    const payload: SessionData = {
      ...data,
      messages: trimmedMessages,
      metadata: {
        ...data.metadata,
        updatedAt: Date.now(),
        messageCount: trimmedMessages.length,
      },
    };

    const jsonStr = JSON.stringify(payload, null, 2);
    const fileName = `${data.metadata.id}.json`;

    // Save to primary (project-level)
    try {
      const projectPath = path.join(this.sessionsDir, fileName);
      fs.writeFileSync(projectPath, jsonStr, 'utf-8');
    } catch (err) {
      process.stderr.write(`[session] Failed to save project session: ${err}\n`);
    }

    // Save to global (backup — always ~/.openanalyst/sessions/)
    if (this.sessionsDir !== this.globalSessionsDir) {
      try {
        fs.mkdirSync(this.globalSessionsDir, { recursive: true });
        const globalPath = path.join(this.globalSessionsDir, fileName);
        fs.writeFileSync(globalPath, jsonStr, 'utf-8');
      } catch {
        // Best-effort — global backup is nice-to-have
      }
    }
  }

  /** Load a session by its full ID. Returns null if not found or corrupted. */
  load(sessionId: string): SessionData | null {
    const filePath = path.join(this.sessionsDir, `${sessionId}.json`);
    return this.readSessionFile(filePath);
  }

  /** Load the most recently updated session. */
  getLatest(): SessionData | null {
    const sessions = this.listSessions();
    if (sessions.length === 0) return null;

    // Already sorted newest-first by listSessions
    return this.load(sessions[0]!.id);
  }

  /** List all sessions, metadata only, sorted newest-first. */
  listSessions(): SessionMetadata[] {
    let files: string[];
    try {
      files = fs.readdirSync(this.sessionsDir).filter((f) => f.endsWith('.json'));
    } catch {
      return [];
    }

    const metadatas: SessionMetadata[] = [];
    for (const file of files) {
      const filePath = path.join(this.sessionsDir, file);
      try {
        const raw = fs.readFileSync(filePath, 'utf-8');
        const parsed = JSON.parse(raw) as SessionData;
        if (parsed?.metadata?.id) {
          metadatas.push(parsed.metadata);
        }
      } catch {
        // Skip corrupted files
        continue;
      }
    }

    // Sort newest first
    metadatas.sort((a, b) => b.updatedAt - a.updatedAt);
    return metadatas;
  }

  /** Delete a session by ID. */
  delete(sessionId: string): boolean {
    const filePath = path.join(this.sessionsDir, `${sessionId}.json`);
    try {
      fs.unlinkSync(filePath);
      return true;
    } catch {
      return false;
    }
  }

  /** Generate a new unique session ID. */
  static newId(): string {
    const ts = Date.now();
    const rand = Math.random().toString(36).slice(2, 8);
    return `session-${ts}-${rand}`;
  }

  /**
   * Sanitize loaded messages for safe replay:
   * - Streaming assistant messages get `streaming: false`
   * - Running tool calls get `status: 'failed'`
   */
  static sanitizeForResume(messages: ChatMessage[]): ChatMessage[] {
    return messages.map((msg) => {
      if (msg.kind === 'assistant' && (msg as AssistantChatMessage).streaming) {
        return { ...msg, streaming: false } as AssistantChatMessage;
      }
      if (msg.kind === 'tool_call' && (msg as ToolCallChatMessage).status === 'running') {
        return { ...msg, status: 'failed' as const } as ToolCallChatMessage;
      }
      return msg;
    });
  }

  // ── Internals ───────────────────────────────────────────────────────────

  /**
   * Resolve sessions directory.
   * Prefer project-local: cwd/.openanalyst/sessions/
   * Fallback to global: ~/.openanalyst/sessions/
   */
  private resolveSessionsDir(): string {
    const projectDir = path.join(process.cwd(), SESSION_DIR_NAME, SESSION_SUB_DIR);
    const globalDir = path.join(os.homedir(), SESSION_DIR_NAME, SESSION_SUB_DIR);

    // If project-local dir already exists, use it
    if (fs.existsSync(path.join(process.cwd(), SESSION_DIR_NAME))) {
      return projectDir;
    }

    // If we can write to cwd, prefer project-local
    try {
      fs.accessSync(process.cwd(), fs.constants.W_OK);
      return projectDir;
    } catch {
      return globalDir;
    }
  }

  /** Ensure both session directories exist. */
  private ensureDir(): void {
    try {
      fs.mkdirSync(this.sessionsDir, { recursive: true });
    } catch {
      // Best-effort
    }
    try {
      fs.mkdirSync(this.globalSessionsDir, { recursive: true });
    } catch {
      // Best-effort
    }
  }

  /** Read and parse a single session file. Returns null on any error. */
  private readSessionFile(filePath: string): SessionData | null {
    try {
      const raw = fs.readFileSync(filePath, 'utf-8');
      const parsed = JSON.parse(raw) as SessionData;
      if (!parsed?.metadata?.id || !Array.isArray(parsed.messages)) {
        return null;
      }
      return parsed;
    } catch {
      return null;
    }
  }
}
