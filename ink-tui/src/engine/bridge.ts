/**
 * EngineBridge — class-based wrapper around the child-process management logic.
 *
 * This is NOT a React component. It manages the Rust engine child process lifecycle
 * (or a mock simulator) and emits typed events via EventEmitter. The EngineProvider
 * context uses this class internally, but it can also be used standalone in tests
 * or non-React scripts.
 *
 * Communication protocol:
 *   - stdin:  TUI -> Engine  (JSON Lines, one action per line)
 *   - stdout: Engine -> TUI  (JSON Lines, one event per line)
 *   - stderr: Engine logs (forwarded as system_message with level: error)
 */

import { spawn, type ChildProcess } from 'node:child_process';
import { createInterface, type Interface as ReadlineInterface } from 'node:readline';
import { EventEmitter } from 'node:events';

import {
  EngineEventSchema,
  type EngineEvent,
  type PermissionMode,
  type ActionCategory,
  type TuiAction,
} from '../types/messages.js';

import { type ConnectionState } from '../types/protocol.js';

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

export interface BridgeConfig {
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
// Event names (typed)
// ---------------------------------------------------------------------------

export interface BridgeEvents {
  event: [EngineEvent];
  connection_state: [ConnectionState];
  parse_error: [line: string, error: unknown];
  exit: [code: number | null, signal: string | null];
}

// ---------------------------------------------------------------------------
// Timestamp helper
// ---------------------------------------------------------------------------

function now(): number {
  return Date.now();
}

// ---------------------------------------------------------------------------
// Action serializer
// ---------------------------------------------------------------------------

function buildAction<T extends TuiAction['type']>(
  type: T,
  payload: Omit<Extract<TuiAction, { type: T }>, 'type' | 'timestamp'>,
): string {
  const message = { type, timestamp: now(), ...payload };
  return JSON.stringify(message) + '\n';
}

// ---------------------------------------------------------------------------
// EngineBridge class
// ---------------------------------------------------------------------------

export class EngineBridge extends EventEmitter {
  private proc: ChildProcess | null = null;
  private rl: ReadlineInterface | null = null;
  private mockEmitter: EventEmitter | null = null;
  private restartCount = 0;
  private _connectionState: ConnectionState = 'disconnected';
  private _config: BridgeConfig;
  private disposed = false;

  constructor(config: BridgeConfig = {}) {
    super();
    this._config = config;
  }

  // -- Public getters -------------------------------------------------------

  get isConnected(): boolean {
    return this._connectionState === 'connected';
  }

  get connectionState(): ConnectionState {
    return this._connectionState;
  }

  // -- Lifecycle ------------------------------------------------------------

  start(): void {
    if (this.disposed) return;
    if (this._config.mock) {
      this.startMock();
    } else {
      this.spawnEngine();
    }
  }

  stop(): void {
    this.disposed = true;
    this.rl?.close();
    this.rl = null;

    if (this.proc && !this.proc.killed) {
      this.proc.kill('SIGTERM');
      const p = this.proc;
      setTimeout(() => {
        if (!p.killed) p.kill('SIGKILL');
      }, 2000);
    }
    this.proc = null;

    this.mockEmitter?.removeAllListeners();
    this.mockEmitter = null;

    this.setConnectionState('disconnected');
  }

  restart(): void {
    this.disposed = false;
    this.stop();
    this.restartCount = 0;
    this.disposed = false;
    this.start();
  }

  dispose(): void {
    this.stop();
    this.removeAllListeners();
  }

  // -- Send methods ---------------------------------------------------------

  /** Send a raw JSON line to the engine. */
  send(jsonLine: string): void {
    if (this._config.mock && this.mockEmitter) {
      this.mockEmitter.emit('action', jsonLine);
      return;
    }
    if (this.proc?.stdin?.writable) {
      this.proc.stdin.write(jsonLine);
    }
  }

  /** Send a typed action. */
  sendAction<T extends TuiAction['type']>(
    type: T,
    payload: Omit<Extract<TuiAction, { type: T }>, 'type' | 'timestamp'>,
  ): void {
    this.send(buildAction(type, payload));
  }

  // -- Convenience action methods -------------------------------------------

  submitPrompt(text: string, opts?: { effortBudget?: number; modelOverride?: string }): void {
    this.sendAction('submit_prompt', { text, ...opts });
  }

  cancelAgent(agentId?: string): void {
    this.sendAction('cancel_agent', { agentId });
  }

  resolvePermission(requestId: string, decision: 'allow' | 'deny'): void {
    this.sendAction('resolve_permission', { requestId, decision });
  }

  resolveAskUser(requestId: string, answer: string): void {
    this.sendAction('resolve_ask_user', { requestId, answer });
  }

  sendKbFeedback(queryId: number, rating: 'positive' | 'negative' | 'corrected', comment?: string, correction?: string): void {
    this.sendAction('kb_feedback', { queryId, rating, comment, correction });
  }

  changePermissionMode(mode: PermissionMode): void {
    this.sendAction('change_permission_mode', { mode });
  }

  toggleContextFile(path: string, action: 'add' | 'remove'): void {
    this.sendAction('toggle_context_file', { path, action });
  }

  changeRouting(category: ActionCategory, tier: 'fast' | 'balanced' | 'capable'): void {
    this.sendAction('change_routing', { category, tier });
  }

  clearChat(): void {
    this.sendAction('clear_chat', {});
  }

  slashCommand(command: string): void {
    this.sendAction('slash_command', { command });
  }

  updateModel(model: string): void {
    this.sendAction('update_model', { model });
  }

  moeDispatch(commands: string[]): void {
    this.sendAction('moe_dispatch', { commands });
  }

  injectSkill(command: string): void {
    this.sendAction('inject_skill', { command });
  }

  quit(): void {
    this.sendAction('quit', {});
  }

  // -- Internal: process management -----------------------------------------

  private setConnectionState(state: ConnectionState): void {
    if (this._connectionState === state) return;
    this._connectionState = state;
    this.emit('connection_state', state);
  }

  private processLine(line: string): void {
    const trimmed = line.trim();
    if (!trimmed) return;

    try {
      const json = JSON.parse(trimmed);
      const result = EngineEventSchema.safeParse(json);
      if (result.success) {
        this.emit('event', result.data);
      } else {
        this.emit('parse_error', trimmed, result.error);
      }
    } catch (err) {
      this.emit('parse_error', trimmed, err);
    }
  }

  private spawnEngine(): void {
    const cfg = this._config;
    const binaryPath = cfg.binaryPath ?? 'openanalyst';
    const args = cfg.args ?? ['--json-rpc'];

    this.setConnectionState('connecting');

    const proc = spawn(binaryPath, args, {
      cwd: cfg.cwd,
      env: { ...process.env, ...cfg.env },
      stdio: ['pipe', 'pipe', 'pipe'],
    });

    this.proc = proc;

    // Read stdout line-by-line
    const rl = createInterface({ input: proc.stdout! });
    this.rl = rl;
    rl.on('line', (line) => this.processLine(line));

    // Stderr -> error system messages
    if (proc.stderr) {
      const stderrRl = createInterface({ input: proc.stderr });
      stderrRl.on('line', (line) => {
        this.emit('event', {
          type: 'system_message',
          timestamp: now(),
          content: line,
          level: 'error',
        } satisfies EngineEvent);
      });
    }

    proc.on('spawn', () => {
      this.setConnectionState('connected');
      this.restartCount = 0;
    });

    proc.on('error', (err) => {
      this.setConnectionState('error');
      this.emit('event', {
        type: 'system_message',
        timestamp: now(),
        content: `Engine process error: ${err.message}`,
        level: 'error',
      } satisfies EngineEvent);
    });

    proc.on('exit', (code, signal) => {
      rl.close();
      this.rl = null;
      this.proc = null;

      this.emit('exit', code, signal);

      if (code !== 0 && code !== null) {
        this.setConnectionState('error');
        this.emit('event', {
          type: 'system_message',
          timestamp: now(),
          content: `Engine exited with code ${code}${signal ? ` (signal: ${signal})` : ''}`,
          level: 'error',
        } satisfies EngineEvent);

        // Auto-restart logic
        const maxAttempts = cfg.maxRestartAttempts ?? 3;
        if (cfg.autoRestart && !this.disposed && this.restartCount < maxAttempts) {
          this.restartCount += 1;
          this.emit('event', {
            type: 'system_message',
            timestamp: now(),
            content: `Auto-restarting engine (attempt ${this.restartCount}/${maxAttempts})...`,
            level: 'warning',
          } satisfies EngineEvent);
          setTimeout(() => this.spawnEngine(), 1000 * this.restartCount);
        }
      } else {
        this.setConnectionState('disconnected');
      }
    });
  }

  // -- Internal: mock engine ------------------------------------------------

  private startMock(): void {
    const emitter = new EventEmitter();
    this.mockEmitter = emitter;

    this.setConnectionState('connected');

    // Emit a banner on start
    setTimeout(() => {
      if (this.disposed) return;
      this.emit('event', {
        type: 'banner',
        timestamp: now(),
        version: '2.0.10-dev',
        displayName: 'Developer',
        email: 'dev@openanalyst.ai',
        provider: 'OpenAnalyst Inc',
        modelDisplay: 'oa-4-turbo (mock)',
        workingDir: process.cwd(),
        tips: [
          'This is mock mode -- no real engine is running',
          'Type a prompt to see simulated streaming',
          'Ctrl+P to cycle permission modes',
        ],
      } satisfies EngineEvent);
    }, 100);

    // React to TUI actions in mock mode
    emitter.on('action', (jsonLine: string) => {
      try {
        const action = JSON.parse(jsonLine) as Record<string, unknown>;
        this.handleMockAction(action);
      } catch {
        // Ignore parse errors in mock mode
      }
    });
  }

  private handleMockAction(action: Record<string, unknown>): void {
    const dispatch = (event: EngineEvent) => this.emit('event', event);

    switch (action.type) {
      case 'submit_prompt': {
        const text = (action.text as string) ?? '';
        this.simulateMockResponse(text);
        break;
      }
      case 'cancel_agent':
        dispatch({ type: 'system_message', timestamp: now(), content: 'Agent cancelled (mock)', level: 'info' });
        dispatch({ type: 'status_update', timestamp: now(), phase: 'idle', elapsedMs: 0 });
        break;
      case 'resolve_permission':
        dispatch({ type: 'system_message', timestamp: now(), content: `Permission ${action.decision}ed (mock)`, level: 'info' });
        break;
      case 'resolve_ask_user':
        dispatch({ type: 'system_message', timestamp: now(), content: `User responded: "${action.answer}" (mock)`, level: 'info' });
        break;
      case 'clear_chat':
        dispatch({ type: 'system_message', timestamp: now(), content: 'Chat cleared (mock)', level: 'info' });
        break;
      default:
        break;
    }
  }

  private simulateMockResponse(prompt: string): void {
    const dispatch = (event: EngineEvent) => this.emit('event', event);
    const agentId = 'mock-primary';
    let elapsed = 0;

    // Phase: thinking
    dispatch({ type: 'status_update', timestamp: now(), phase: 'thinking', elapsedMs: 0 });

    // Simulate a tool call after 300ms
    setTimeout(() => {
      if (this.disposed) return;
      elapsed = 300;
      dispatch({ type: 'tool_call_start', timestamp: now(), agentId, toolId: 'mock-tool-1', toolName: 'Read', inputPreview: 'src/index.ts' });
      dispatch({ type: 'status_update', timestamp: now(), phase: 'reading_file', label: 'index.ts', elapsedMs: elapsed });
    }, 300);

    // Complete tool call after 600ms
    setTimeout(() => {
      if (this.disposed) return;
      elapsed = 600;
      dispatch({ type: 'tool_call_complete', timestamp: now(), agentId, toolId: 'mock-tool-1', status: 'completed', output: '// Entry point\nimport { App } from "./app";\n// ...', durationMs: 280 });
      dispatch({ type: 'status_update', timestamp: now(), phase: 'thinking', elapsedMs: elapsed });
    }, 600);

    // Stream response chunks starting at 800ms
    const responseText = `I've read the file. Here's what I found regarding "${prompt}":\n\nThe project is structured as a standard TypeScript application with React components rendered via Ink for terminal UI.\n\n**Key observations:**\n- Entry point initializes the Ink renderer\n- App component manages the layout tree\n- All communication with the engine happens through JSON-RPC over stdin/stdout`;

    const words = responseText.split(' ');
    let wordIndex = 0;

    const streamInterval = setInterval(() => {
      if (this.disposed) {
        clearInterval(streamInterval);
        return;
      }
      if (wordIndex >= words.length) {
        clearInterval(streamInterval);
        dispatch({ type: 'stream_delta', timestamp: now(), agentId, content: '', done: true });
        dispatch({ type: 'stream_end', timestamp: now(), agentId });
        dispatch({ type: 'status_update', timestamp: now(), phase: 'done', elapsedMs: elapsed });
        dispatch({ type: 'usage_update', timestamp: now(), agentId, inputTokens: 1250, outputTokens: words.length * 2 });

        setTimeout(() => {
          if (!this.disposed) {
            dispatch({ type: 'status_update', timestamp: now(), phase: 'idle', elapsedMs: 0 });
          }
        }, 1000);
        return;
      }

      const chunk = (wordIndex === 0 ? '' : ' ') + words[wordIndex]!;
      elapsed += 50;
      dispatch({ type: 'stream_delta', timestamp: now(), agentId, content: chunk, done: false });
      wordIndex++;
    }, 50);
  }
}
