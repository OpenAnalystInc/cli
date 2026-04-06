/**
 * EngineBridge — class-based wrapper around the child-process management logic.
 *
 * This is NOT a React component. It manages the Rust engine child process lifecycle
 * and emits typed events via EventEmitter. The EngineProvider context uses this
 * class internally, but it can also be used standalone in tests or non-React scripts.
 *
 * Communication protocol:
 *   - stdin:  TUI -> Engine  (JSON Lines, one action per line)
 *   - stdout: Engine -> TUI  (JSON Lines, one event per line)
 *   - stderr: Engine logs (forwarded as system_message with level: error)
 */

import { spawn, type ChildProcess } from 'node:child_process';
import { createInterface, type Interface as ReadlineInterface } from 'node:readline';
import { EventEmitter } from 'node:events';
import { userInfo } from 'node:os';

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
    this.spawnEngine();
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
    // Rust expects "permission_response" with { requestId, allow: boolean }
    this.sendAction('permission_response', { requestId, allow: decision === 'allow' });
  }

  resolveAskUser(requestId: string, answer: string): void {
    // Rust expects "ask_user_response" with { requestId, response }
    this.sendAction('ask_user_response', { requestId, response: answer });
  }

  sendKbFeedback(queryId: number, rating: 'positive' | 'negative' | 'corrected', comment?: string, correction?: string): void {
    // Rust expects "knowledge_feedback" with required comment/correction strings
    this.sendAction('knowledge_feedback', { queryId, rating, comment: comment ?? '', correction: correction ?? '' });
  }

  changePermissionMode(mode: PermissionMode): void {
    // Rust expects "update_permissions" (newtype variant -- may not deserialize correctly)
    this.sendAction('update_permissions', { mode });
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
    const args = cfg.args ?? ['--headless'];

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

      // Emit banner and initial sidebar with real user data
      (async () => {
        try {
          const { credentialManager, PROVIDER_CONFIG } = await import('../utils/credential-manager.js');
          const { providerPreferences } = await import('../utils/provider-preferences.js');
          const { fetchCredits } = await import('../utils/credit-checker.js');

          const defaultProvider = providerPreferences.getDefaultProvider();
          const models = providerPreferences.getModelsForProvider(defaultProvider || '');

          const fastModel = models.find(m => m.tier === 'fast') ?? null;
          const balancedModel = models.find(m => m.tier === 'balanced') ?? null;
          const capableModel = models.find(m => m.tier === 'capable') ?? null;

          const defaultModel = balancedModel ?? capableModel ?? fastModel ?? null;

          const providerConfig = defaultProvider ? PROVIDER_CONFIG[defaultProvider] : null;
          const providerDisplayName = providerConfig?.displayName || 'Not configured';

          let creditStr = 'No API key configured';
          try {
            const creditInfo = await fetchCredits();
            if (creditInfo.provider !== 'unknown') {
              creditStr = creditInfo.balance;
            }
          } catch {
            // Keep default
          }

          const modelDisplay = defaultModel
            ? defaultModel.name
            : 'Run /login to configure';

          const displayName = process.env['USER'] || process.env['USERNAME'] || userInfo().username || 'User';

          const orgName = defaultProvider === 'openanalyst'
            ? 'OpenAnalyst Inc'
            : providerDisplayName;

          this.emit('event', {
            type: 'banner',
            timestamp: now(),
            version: '2.0.12',
            displayName,
            organization: orgName,
            provider: orgName,
            modelDisplay,
            credits: creditStr,
            workingDir: process.cwd(),
            tips: defaultProvider
              ? [
                  `/model to switch AI models`,
                  `Provider: ${providerDisplayName}`,
                  'Ctrl+E to toggle sidebar',
                ]
              : [
                  'Run /login <provider> <key>',
                  'OpenAI, Anthropic, Gemini, xAI',
                  'Run /help for all commands',
                ],
          } satisfies EngineEvent);

          this.emit('event', {
            type: 'sidebar_update',
            timestamp: now(),
            agents: [],
            files: [],
            plans: [],
            routing: {
              explore:  { model: fastModel?.name     || 'none', tier: 'fast' },
              research: { model: balancedModel?.name || 'none', tier: 'balanced' },
              code:     { model: balancedModel?.name || 'none', tier: 'balanced' },
              write:    { model: capableModel?.name  || 'none', tier: 'capable' },
            },
            activity: {
              backgroundTasks: 0,
              toolCallCount: 0,
              mcpServers: 0,
              creditBalance: creditStr,
            },
          } satisfies EngineEvent);

          this.emit('event', {
            type: 'status_update',
            timestamp: now(),
            phase: 'idle',
            elapsedMs: 0,
          } satisfies EngineEvent);
        } catch {
          // Banner generation failed — engine is still connected, just no banner
        }
      })();
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

}
