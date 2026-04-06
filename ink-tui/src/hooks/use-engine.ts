/**
 * useEngine() — React hook that manages bidirectional JSON-line communication
 * with the Rust engine child process.
 *
 * Features:
 * - Spawns the Rust binary as a child process (path configurable)
 * - Reads stdout line-by-line, parses JSON, validates with Zod schemas
 * - Dispatches parsed events to registered handlers
 * - Provides typed action methods (sendPrompt, cancelAgent, etc.)
 * - Handles process crash with optional auto-restart
 * - Tracks connection state: connecting | connected | disconnected | error
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { spawn, type ChildProcess } from 'node:child_process';
import { createInterface, type Interface as ReadlineInterface } from 'node:readline';

import {
  EngineEventSchema,
  type EngineEvent,
  type StreamDelta,
  type StreamEnd,
  type ToolCallStart,
  type ToolCallUpdate,
  type ToolCallEnd,
  type PermissionRequest,
  type AskUserRequest,
  type StatusUpdate,
  type AgentSpawned,
  type AgentStatusChanged,
  type AgentCompleted,
  type AgentFailed,
  type UsageUpdate,
  type KbResult,
  type SystemMessage,
  type Banner,
  type SidebarUpdate,
  type ModelInfo,
  type ContextFilesUpdate,
  type PermissionMode,
  type ActionCategory,
  type TuiAction,
} from '../types/messages.js';

import { type ConnectionState } from '../types/protocol.js';

// ---------------------------------------------------------------------------
// Event handler map
// ---------------------------------------------------------------------------

export interface EngineEventHandlers {
  onStreamDelta?: (event: StreamDelta) => void;
  onStreamEnd?: (event: StreamEnd) => void;
  onToolCallStart?: (event: ToolCallStart) => void;
  onToolCallUpdate?: (event: ToolCallUpdate) => void;
  onToolCallComplete?: (event: ToolCallEnd) => void;
  onPermissionRequest?: (event: PermissionRequest) => void;
  onAskUserRequest?: (event: AskUserRequest) => void;
  onStatusUpdate?: (event: StatusUpdate) => void;
  onAgentSpawned?: (event: AgentSpawned) => void;
  onAgentStatusChanged?: (event: AgentStatusChanged) => void;
  onAgentCompleted?: (event: AgentCompleted) => void;
  onAgentFailed?: (event: AgentFailed) => void;
  onUsageUpdate?: (event: UsageUpdate) => void;
  onKbResult?: (event: KbResult) => void;
  onSystemMessage?: (event: SystemMessage) => void;
  onBanner?: (event: Banner) => void;
  onSidebarUpdate?: (event: SidebarUpdate) => void;
  onModelInfo?: (event: ModelInfo) => void;
  onContextFilesUpdate?: (event: ContextFilesUpdate) => void;
  onConnectionStateChange?: (state: ConnectionState) => void;
  onParseError?: (line: string, error: unknown) => void;
}

// ---------------------------------------------------------------------------
// Engine configuration
// ---------------------------------------------------------------------------

export interface EngineConfig {
  /** Path to the Rust engine binary. Defaults to 'openanalyst'. */
  binaryPath?: string;
  /** Arguments to pass to the engine. */
  args?: string[];
  /** Working directory for the engine process. */
  cwd?: string;
  /** Environment variables to set on the engine process. */
  env?: Record<string, string>;
  /** Auto-restart on crash. Defaults to false. */
  autoRestart?: boolean;
  /** Max restart attempts before giving up. Defaults to 3. */
  maxRestartAttempts?: number;
}

// ---------------------------------------------------------------------------
// Timestamp helper
// ---------------------------------------------------------------------------

function now(): number {
  return Date.now();
}

// ---------------------------------------------------------------------------
// Action builder helpers
// ---------------------------------------------------------------------------

function buildAction<T extends TuiAction['type']>(
  type: T,
  payload: Omit<Extract<TuiAction, { type: T }>, 'type' | 'timestamp'>,
): string {
  const message = { type, timestamp: now(), ...payload };
  return JSON.stringify(message) + '\n';
}

// ---------------------------------------------------------------------------
// The hook
// ---------------------------------------------------------------------------

export interface UseEngineReturn {
  /** Current connection state. */
  connectionState: ConnectionState;

  /** Send a user prompt to the engine. */
  sendPrompt: (text: string, opts?: { effortBudget?: number; modelOverride?: string }) => void;

  /** Submit a prompt to run in the background. */
  runInBackground: (text: string) => void;

  /** Cancel a running agent (current agent if no ID specified). */
  cancelAgent: (agentId?: string) => void;

  /** Resolve a permission request. */
  resolvePermission: (requestId: string, decision: 'allow' | 'deny') => void;

  /** Resolve an ask-user dialog. */
  resolveAskUser: (requestId: string, answer: string) => void;

  /** Submit knowledge base feedback. */
  sendKbFeedback: (queryId: number, rating: 'positive' | 'negative' | 'corrected', comment?: string, correction?: string) => void;

  /** Change permission mode (Ctrl+P cycle). */
  changePermissionMode: (mode: PermissionMode) => void;

  /** Toggle a context file. */
  toggleContextFile: (path: string, action: 'add' | 'remove') => void;

  /** Change routing for an action category. */
  changeRouting: (category: ActionCategory, tier: 'fast' | 'balanced' | 'capable') => void;

  /** Clear chat (Ctrl+L). */
  clearChat: () => void;

  /** Send a slash command. */
  slashCommand: (command: string) => void;

  /** Change the default model. */
  updateModel: (model: string) => void;

  /** Dispatch parallel agent commands (MOE). */
  moeDispatch: (commands: string[]) => void;

  /** Inject a skill while agents are working. */
  injectSkill: (command: string) => void;

  /** Tell the engine to quit. */
  quit: () => void;

  /** Manually restart the engine process. */
  restart: () => void;
}

export function useEngine(
  config: EngineConfig = {},
  handlers: EngineEventHandlers = {},
): UseEngineReturn {
  const [connectionState, setConnectionState] = useState<ConnectionState>('disconnected');
  const processRef = useRef<ChildProcess | null>(null);
  const rlRef = useRef<ReadlineInterface | null>(null);
  const restartCountRef = useRef(0);
  const handlersRef = useRef(handlers);
  const configRef = useRef(config);

  // Keep refs up to date
  handlersRef.current = handlers;
  configRef.current = config;

  // -- Dispatch parsed event to appropriate handler --
  const dispatchEvent = useCallback((event: EngineEvent) => {
    const h = handlersRef.current;
    switch (event.type) {
      case 'stream_delta': return h.onStreamDelta?.(event);
      case 'stream_end': return h.onStreamEnd?.(event);
      case 'tool_call_start': return h.onToolCallStart?.(event);
      case 'tool_call_update': return h.onToolCallUpdate?.(event);
      case 'tool_call_end': return h.onToolCallComplete?.(event);
      case 'permission_request': return h.onPermissionRequest?.(event);
      case 'ask_user_request': return h.onAskUserRequest?.(event);
      case 'status_update': return h.onStatusUpdate?.(event);
      case 'agent_spawned': return h.onAgentSpawned?.(event);
      case 'agent_status_changed': return h.onAgentStatusChanged?.(event);
      case 'agent_completed': return h.onAgentCompleted?.(event);
      case 'agent_failed': return h.onAgentFailed?.(event);
      case 'usage_update': return h.onUsageUpdate?.(event);
      case 'knowledge_result': return h.onKbResult?.(event);
      case 'system_message': return h.onSystemMessage?.(event);
      case 'banner': return h.onBanner?.(event);
      case 'sidebar_update': return h.onSidebarUpdate?.(event);
      case 'model_info': return h.onModelInfo?.(event);
      case 'context_files_update': return h.onContextFilesUpdate?.(event);
    }
  }, []);

  // -- Send a raw JSON line to the engine stdin --
  const send = useCallback((jsonLine: string) => {
    const proc = processRef.current;
    if (proc?.stdin?.writable) {
      proc.stdin.write(jsonLine);
    }
  }, []);

  // -- Process a single line from stdout --
  const processLine = useCallback(
    (line: string) => {
      const trimmed = line.trim();
      if (!trimmed) return;

      try {
        const json = JSON.parse(trimmed);
        const result = EngineEventSchema.safeParse(json);
        if (result.success) {
          dispatchEvent(result.data);
        } else {
          handlersRef.current.onParseError?.(trimmed, result.error);
        }
      } catch (err) {
        handlersRef.current.onParseError?.(trimmed, err);
      }
    },
    [dispatchEvent],
  );

  // -- Spawn the real engine process --
  const spawnEngine = useCallback(() => {
    const cfg = configRef.current;
    const binaryPath = cfg.binaryPath ?? 'openanalyst';
    const args = cfg.args ?? ['--headless'];

    setConnectionState('connecting');
    handlersRef.current.onConnectionStateChange?.('connecting');

    const proc = spawn(binaryPath, args, {
      cwd: cfg.cwd,
      env: { ...process.env, ...cfg.env },
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    processRef.current = proc;

    // Read stdout line-by-line
    const rl = createInterface({ input: proc.stdout! });
    rlRef.current = rl;
    rl.on('line', processLine);

    // Stderr -> system error messages
    if (proc.stderr) {
      const stderrRl = createInterface({ input: proc.stderr });
      stderrRl.on('line', (line) => {
        // Stderr lines are treated as engine-side logs — forward as error system messages
        dispatchEvent({
          type: 'system_message',
          timestamp: now(),
          content: line,
          level: 'error',
        });
      });
    }

    proc.on('spawn', () => {
      setConnectionState('connected');
      handlersRef.current.onConnectionStateChange?.('connected');
      restartCountRef.current = 0;
    });

    proc.on('error', (err) => {
      setConnectionState('error');
      handlersRef.current.onConnectionStateChange?.('error');
      dispatchEvent({
        type: 'system_message',
        timestamp: now(),
        content: `Engine process error: ${err.message}`,
        level: 'error',
      });
    });

    proc.on('exit', (code, signal) => {
      rl.close();
      rlRef.current = null;
      processRef.current = null;

      if (code !== 0 && code !== null) {
        setConnectionState('error');
        handlersRef.current.onConnectionStateChange?.('error');
        dispatchEvent({
          type: 'system_message',
          timestamp: now(),
          content: `Engine exited with code ${code}${signal ? ` (signal: ${signal})` : ''}`,
          level: 'error',
        });

        // Auto-restart logic
        const maxAttempts = cfg.maxRestartAttempts ?? 3;
        if (cfg.autoRestart && restartCountRef.current < maxAttempts) {
          restartCountRef.current += 1;
          dispatchEvent({
            type: 'system_message',
            timestamp: now(),
            content: `Auto-restarting engine (attempt ${restartCountRef.current}/${maxAttempts})...`,
            level: 'warning',
          });
          setTimeout(spawnEngine, 1000 * restartCountRef.current); // backoff
        }
      } else {
        setConnectionState('disconnected');
        handlersRef.current.onConnectionStateChange?.('disconnected');
      }
    });
  }, [processLine, dispatchEvent]);

  // -- Lifecycle: spawn on mount, kill on unmount --
  useEffect(() => {
    spawnEngine();

    return () => {
      // Cleanup
      rlRef.current?.close();
      const proc = processRef.current;
      if (proc && !proc.killed) {
        proc.kill('SIGTERM');
        // Force kill after 2 seconds
        setTimeout(() => {
          if (!proc.killed) proc.kill('SIGKILL');
        }, 2000);
      }
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // -- Public action methods --

  const sendPrompt = useCallback(
    (text: string, opts?: { effortBudget?: number; modelOverride?: string }) => {
      send(buildAction('submit_prompt', { text, ...opts }));
    },
    [send],
  );

  const runInBackground = useCallback(
    (text: string) => send(buildAction('run_in_background', { text })),
    [send],
  );

  const cancelAgent = useCallback(
    (agentId?: string) => send(buildAction('cancel_agent', { agentId })),
    [send],
  );

  const resolvePermission = useCallback(
    (requestId: string, decision: 'allow' | 'deny') =>
      send(buildAction('permission_response', { requestId, allow: decision === 'allow' })),
    [send],
  );

  const resolveAskUser = useCallback(
    (requestId: string, answer: string) =>
      send(buildAction('ask_user_response', { requestId, response: answer })),
    [send],
  );

  const sendKbFeedback = useCallback(
    (queryId: number, rating: 'positive' | 'negative' | 'corrected', comment?: string, correction?: string) =>
      send(buildAction('knowledge_feedback', { queryId, rating, comment: comment ?? '', correction: correction ?? '' })),
    [send],
  );

  const changePermissionMode = useCallback(
    (mode: PermissionMode) => send(buildAction('update_permissions', { mode })),
    [send],
  );

  const toggleContextFile = useCallback(
    (path: string, action: 'add' | 'remove') =>
      send(buildAction('toggle_context_file', { path, action })),
    [send],
  );

  const changeRouting = useCallback(
    (category: ActionCategory, tier: 'fast' | 'balanced' | 'capable') =>
      send(buildAction('change_routing', { category, tier })),
    [send],
  );

  const clearChat = useCallback(
    () => send(buildAction('clear_chat', {})),
    [send],
  );

  const slashCommand = useCallback(
    (command: string) => send(buildAction('slash_command', { command })),
    [send],
  );

  const updateModel = useCallback(
    (model: string) => send(buildAction('update_model', { model })),
    [send],
  );

  const moeDispatch = useCallback(
    (commands: string[]) => send(buildAction('moe_dispatch', { commands })),
    [send],
  );

  const injectSkill = useCallback(
    (command: string) => send(buildAction('inject_skill', { command })),
    [send],
  );

  const quit = useCallback(
    () => send(buildAction('quit', {})),
    [send],
  );

  const restart = useCallback(() => {
    // Kill existing process
    const proc = processRef.current;
    if (proc && !proc.killed) {
      proc.kill('SIGTERM');
    }
    rlRef.current?.close();

    restartCountRef.current = 0;
    spawnEngine();
  }, [spawnEngine]);

  return {
    connectionState,
    sendPrompt,
    runInBackground,
    cancelAgent,
    resolvePermission,
    resolveAskUser,
    sendKbFeedback,
    changePermissionMode,
    toggleContextFile,
    changeRouting,
    clearChat,
    slashCommand,
    updateModel,
    moeDispatch,
    injectSkill,
    quit,
    restart,
  };
}

