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
 * - Mock mode for UI development without a real engine
 */

import { useCallback, useEffect, useRef, useState } from 'react';
import { spawn, type ChildProcess } from 'node:child_process';
import { createInterface, type Interface as ReadlineInterface } from 'node:readline';
import { EventEmitter } from 'node:events';

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
  /** If true, use mock engine instead of a real process. */
  mock?: boolean;
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
  const mockEmitterRef = useRef<EventEmitter | null>(null);

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
    if (configRef.current.mock && mockEmitterRef.current) {
      // In mock mode, emit to the mock emitter so the mock engine can react
      mockEmitterRef.current.emit('action', jsonLine);
      return;
    }
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
    if (cfg.mock) return; // Don't spawn in mock mode

    const binaryPath = cfg.binaryPath ?? 'openanalyst';
    const args = cfg.args ?? ['--json-rpc'];

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

  // -- Start mock engine --
  const startMock = useCallback(() => {
    const emitter = new EventEmitter();
    mockEmitterRef.current = emitter;

    setConnectionState('connected');
    handlersRef.current.onConnectionStateChange?.('connected');

    // Emit a banner on start
    setTimeout(() => {
      dispatchEvent({
        type: 'banner',
        timestamp: now(),
        version: '2.0.12',
        displayName: 'Developer',
        email: 'dev@openanalyst.ai',
        provider: 'OpenAnalyst Inc',
        modelDisplay: 'oa-4-turbo (mock)',
        workingDir: process.cwd(),
        tips: [
          'This is mock mode — no real engine is running',
          'Type a prompt to see simulated streaming',
          'Ctrl+P to cycle permission modes',
        ],
      });
    }, 100);

    // React to TUI actions in mock mode
    emitter.on('action', (jsonLine: string) => {
      try {
        const action = JSON.parse(jsonLine);
        handleMockAction(action, dispatchEvent);
      } catch {
        // Ignore parse errors in mock mode
      }
    });
  }, [dispatchEvent]);

  // -- Lifecycle: spawn on mount, kill on unmount --
  useEffect(() => {
    if (configRef.current.mock) {
      startMock();
    } else {
      spawnEngine();
    }

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
      mockEmitterRef.current?.removeAllListeners();
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
    mockEmitterRef.current?.removeAllListeners();

    restartCountRef.current = 0;
    if (configRef.current.mock) {
      startMock();
    } else {
      spawnEngine();
    }
  }, [spawnEngine, startMock]);

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

// ═══════════════════════════════════════════════════════════════════════════
// Mock engine — simulates events for UI development
// ═══════════════════════════════════════════════════════════════════════════

type DispatchFn = (event: EngineEvent) => void;

function handleMockAction(action: Record<string, unknown>, dispatch: DispatchFn): void {
  switch (action.type) {
    case 'submit_prompt': {
      const text = (action.text as string) ?? '';
      simulateResponse(text, dispatch);
      break;
    }
    case 'cancel_agent': {
      dispatch({
        type: 'system_message',
        timestamp: now(),
        content: 'Agent cancelled (mock)',
        level: 'info',
      });
      dispatch({
        type: 'status_update',
        timestamp: now(),
        phase: 'idle',
        elapsedMs: 0,
      });
      break;
    }
    case 'permission_response': {
      dispatch({
        type: 'system_message',
        timestamp: now(),
        content: `Permission ${action.allow ? 'allow' : 'deny'}ed (mock)`,
        level: 'info',
      });
      break;
    }
    case 'ask_user_response': {
      dispatch({
        type: 'system_message',
        timestamp: now(),
        content: `User responded: "${action.response}" (mock)`,
        level: 'info',
      });
      break;
    }
    case 'clear_chat': {
      dispatch({
        type: 'system_message',
        timestamp: now(),
        content: 'Chat cleared (mock)',
        level: 'info',
      });
      break;
    }
    default:
      break;
  }
}

/**
 * Simulate a full response cycle: thinking -> tool call -> streaming -> done.
 */
function simulateResponse(prompt: string, dispatch: DispatchFn): void {
  const agentId = 'mock-primary';
  let elapsed = 0;

  // Phase: thinking
  dispatch({
    type: 'status_update',
    timestamp: now(),
    phase: 'thinking',
    elapsedMs: 0,
  });

  // Simulate a tool call after 300ms
  setTimeout(() => {
    elapsed = 300;
    dispatch({
      type: 'tool_call_start',
      timestamp: now(),
      agentId,
      callId: 'mock-tool-1',
      toolName: 'Read',
      inputPreview: 'src/index.ts',
    });
    dispatch({
      type: 'status_update',
      timestamp: now(),
      phase: 'reading_file',
      label: 'index.ts',
      elapsedMs: elapsed,
    });
  }, 300);

  // Complete tool call after 600ms
  setTimeout(() => {
    elapsed = 600;
    dispatch({
      type: 'tool_call_end',
      timestamp: now(),
      agentId,
      callId: 'mock-tool-1',
      isError: false,
      output: '// Entry point\nimport { App } from "./app";\n// ...',
      duration: 280,
    });
    dispatch({
      type: 'status_update',
      timestamp: now(),
      phase: 'thinking',
      elapsedMs: elapsed,
    });
  }, 600);

  // Stream response chunks starting at 800ms
  const responseText = `I've read the file. Here's what I found regarding "${prompt}":\n\nThe project is structured as a standard TypeScript application with React components rendered via Ink for terminal UI.\n\n**Key observations:**\n- Entry point initializes the Ink renderer\n- App component manages the layout tree\n- All communication with the engine happens through JSON-RPC over stdin/stdout`;

  const words = responseText.split(' ');
  let wordIndex = 0;

  const streamInterval = setInterval(() => {
    if (wordIndex >= words.length) {
      clearInterval(streamInterval);
      // Stream end
      dispatch({
        type: 'stream_delta',
        timestamp: now(),
        agentId,
        text: '',
      });
      dispatch({
        type: 'stream_end',
        timestamp: now(),
        agentId,
      });
      dispatch({
        type: 'status_update',
        timestamp: now(),
        phase: 'done',
        elapsedMs: elapsed,
      });
      dispatch({
        type: 'usage_update',
        timestamp: now(),
        agentId,
        inputTokens: 1250,
        outputTokens: words.length * 2,
      });

      // Return to idle after 1s
      setTimeout(() => {
        dispatch({
          type: 'status_update',
          timestamp: now(),
          phase: 'idle',
          elapsedMs: 0,
        });
      }, 1000);
      return;
    }

    const chunk = (wordIndex === 0 ? '' : ' ') + words[wordIndex]!;
    elapsed += 50;
    dispatch({
      type: 'stream_delta',
      timestamp: now(),
      agentId,
      text: chunk,
    });
    wordIndex++;
  }, 50);

  // Start streaming after tool call completes
  setTimeout(() => {
    // The interval already started above but won't begin emitting until 800ms
  }, 800);
}

// ═══════════════════════════════════════════════════════════════════════════
// createMockEngine() — standalone factory for non-hook usage
// ═══════════════════════════════════════════════════════════════════════════

export interface MockEngine {
  /** Emit a mock event to registered handlers. */
  emit: (event: EngineEvent) => void;
  /** Register an event handler. Returns unsubscribe function. */
  on: (handler: (event: EngineEvent) => void) => () => void;
  /** Send an action (triggers mock responses). */
  send: (action: TuiAction) => void;
  /** Dispose of the mock engine. */
  dispose: () => void;
}

/**
 * Create a standalone mock engine instance for testing and UI development.
 * This is not a React hook — use it in test files or non-React contexts.
 */
export function createMockEngine(): MockEngine {
  const emitter = new EventEmitter();
  const handlers = new Set<(event: EngineEvent) => void>();

  const dispatch: DispatchFn = (event) => {
    for (const handler of handlers) {
      handler(event);
    }
  };

  // Emit banner on creation
  setTimeout(() => {
    dispatch({
      type: 'banner',
      timestamp: now(),
      version: '2.0.12',
      displayName: 'Mock User',
      email: 'mock@openanalyst.ai',
      provider: 'OpenAnalyst Inc',
      modelDisplay: 'oa-4-turbo (mock)',
      workingDir: '/mock/workspace',
      tips: ['Mock engine active', 'Use engine.send() to simulate actions'],
    });
  }, 0);

  return {
    emit: dispatch,

    on(handler) {
      handlers.add(handler);
      return () => {
        handlers.delete(handler);
      };
    },

    send(action) {
      handleMockAction(action as unknown as Record<string, unknown>, dispatch);
    },

    dispose() {
      handlers.clear();
      emitter.removeAllListeners();
    },
  };
}
