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
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { EventEmitter } from 'node:events';
import { EngineEventSchema, } from '../types/messages.js';
// ---------------------------------------------------------------------------
// Timestamp helper
// ---------------------------------------------------------------------------
function now() {
    return Date.now();
}
// ---------------------------------------------------------------------------
// Action serializer
// ---------------------------------------------------------------------------
function buildAction(type, payload) {
    const message = { type, timestamp: now(), ...payload };
    return JSON.stringify(message) + '\n';
}
// ---------------------------------------------------------------------------
// EngineBridge class
// ---------------------------------------------------------------------------
export class EngineBridge extends EventEmitter {
    proc = null;
    rl = null;
    mockEmitter = null;
    restartCount = 0;
    _connectionState = 'disconnected';
    _config;
    disposed = false;
    constructor(config = {}) {
        super();
        this._config = config;
    }
    // -- Public getters -------------------------------------------------------
    get isConnected() {
        return this._connectionState === 'connected';
    }
    get connectionState() {
        return this._connectionState;
    }
    // -- Lifecycle ------------------------------------------------------------
    start() {
        if (this.disposed)
            return;
        if (this._config.mock) {
            this.startMock();
        }
        else {
            this.spawnEngine();
        }
    }
    stop() {
        this.disposed = true;
        this.rl?.close();
        this.rl = null;
        if (this.proc && !this.proc.killed) {
            this.proc.kill('SIGTERM');
            const p = this.proc;
            setTimeout(() => {
                if (!p.killed)
                    p.kill('SIGKILL');
            }, 2000);
        }
        this.proc = null;
        this.mockEmitter?.removeAllListeners();
        this.mockEmitter = null;
        this.setConnectionState('disconnected');
    }
    restart() {
        this.disposed = false;
        this.stop();
        this.restartCount = 0;
        this.disposed = false;
        this.start();
    }
    dispose() {
        this.stop();
        this.removeAllListeners();
    }
    // -- Send methods ---------------------------------------------------------
    /** Send a raw JSON line to the engine. */
    send(jsonLine) {
        if (this._config.mock && this.mockEmitter) {
            this.mockEmitter.emit('action', jsonLine);
            return;
        }
        if (this.proc?.stdin?.writable) {
            this.proc.stdin.write(jsonLine);
        }
    }
    /** Send a typed action. */
    sendAction(type, payload) {
        this.send(buildAction(type, payload));
    }
    // -- Convenience action methods -------------------------------------------
    submitPrompt(text, opts) {
        this.sendAction('submit_prompt', { text, ...opts });
    }
    cancelAgent(agentId) {
        this.sendAction('cancel_agent', { agentId });
    }
    resolvePermission(requestId, decision) {
        this.sendAction('resolve_permission', { requestId, decision });
    }
    resolveAskUser(requestId, answer) {
        this.sendAction('resolve_ask_user', { requestId, answer });
    }
    sendKbFeedback(queryId, rating, comment, correction) {
        this.sendAction('kb_feedback', { queryId, rating, comment, correction });
    }
    changePermissionMode(mode) {
        this.sendAction('change_permission_mode', { mode });
    }
    toggleContextFile(path, action) {
        this.sendAction('toggle_context_file', { path, action });
    }
    changeRouting(category, tier) {
        this.sendAction('change_routing', { category, tier });
    }
    clearChat() {
        this.sendAction('clear_chat', {});
    }
    slashCommand(command) {
        this.sendAction('slash_command', { command });
    }
    updateModel(model) {
        this.sendAction('update_model', { model });
    }
    moeDispatch(commands) {
        this.sendAction('moe_dispatch', { commands });
    }
    injectSkill(command) {
        this.sendAction('inject_skill', { command });
    }
    quit() {
        this.sendAction('quit', {});
    }
    // -- Internal: process management -----------------------------------------
    setConnectionState(state) {
        if (this._connectionState === state)
            return;
        this._connectionState = state;
        this.emit('connection_state', state);
    }
    processLine(line) {
        const trimmed = line.trim();
        if (!trimmed)
            return;
        try {
            const json = JSON.parse(trimmed);
            const result = EngineEventSchema.safeParse(json);
            if (result.success) {
                this.emit('event', result.data);
            }
            else {
                this.emit('parse_error', trimmed, result.error);
            }
        }
        catch (err) {
            this.emit('parse_error', trimmed, err);
        }
    }
    spawnEngine() {
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
        const rl = createInterface({ input: proc.stdout });
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
                });
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
            });
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
                });
                // Auto-restart logic
                const maxAttempts = cfg.maxRestartAttempts ?? 3;
                if (cfg.autoRestart && !this.disposed && this.restartCount < maxAttempts) {
                    this.restartCount += 1;
                    this.emit('event', {
                        type: 'system_message',
                        timestamp: now(),
                        content: `Auto-restarting engine (attempt ${this.restartCount}/${maxAttempts})...`,
                        level: 'warning',
                    });
                    setTimeout(() => this.spawnEngine(), 1000 * this.restartCount);
                }
            }
            else {
                this.setConnectionState('disconnected');
            }
        });
    }
    // -- Internal: mock engine ------------------------------------------------
    startMock() {
        const emitter = new EventEmitter();
        this.mockEmitter = emitter;
        this.setConnectionState('connected');
        // Emit a banner on start
        setTimeout(() => {
            if (this.disposed)
                return;
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
            });
        }, 100);
        // React to TUI actions in mock mode
        emitter.on('action', (jsonLine) => {
            try {
                const action = JSON.parse(jsonLine);
                this.handleMockAction(action);
            }
            catch {
                // Ignore parse errors in mock mode
            }
        });
    }
    handleMockAction(action) {
        const dispatch = (event) => this.emit('event', event);
        switch (action.type) {
            case 'submit_prompt': {
                const text = action.text ?? '';
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
    simulateMockResponse(prompt) {
        const dispatch = (event) => this.emit('event', event);
        const agentId = 'mock-primary';
        let elapsed = 0;
        // Phase: thinking
        dispatch({ type: 'status_update', timestamp: now(), phase: 'thinking', elapsedMs: 0 });
        // Simulate a tool call after 300ms
        setTimeout(() => {
            if (this.disposed)
                return;
            elapsed = 300;
            dispatch({ type: 'tool_call_start', timestamp: now(), agentId, toolId: 'mock-tool-1', toolName: 'Read', inputPreview: 'src/index.ts' });
            dispatch({ type: 'status_update', timestamp: now(), phase: 'reading_file', label: 'index.ts', elapsedMs: elapsed });
        }, 300);
        // Complete tool call after 600ms
        setTimeout(() => {
            if (this.disposed)
                return;
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
            const chunk = (wordIndex === 0 ? '' : ' ') + words[wordIndex];
            elapsed += 50;
            dispatch({ type: 'stream_delta', timestamp: now(), agentId, content: chunk, done: false });
            wordIndex++;
        }, 50);
    }
}
//# sourceMappingURL=bridge.js.map