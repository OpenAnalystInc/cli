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
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { EngineEventSchema, } from '../types/messages.js';
// ---------------------------------------------------------------------------
// Timestamp helper
// ---------------------------------------------------------------------------
function now() {
    return Date.now();
}
// ---------------------------------------------------------------------------
// Action builder helpers
// ---------------------------------------------------------------------------
function buildAction(type, payload) {
    const message = { type, timestamp: now(), ...payload };
    return JSON.stringify(message) + '\n';
}
export function useEngine(config = {}, handlers = {}) {
    const [connectionState, setConnectionState] = useState('disconnected');
    const processRef = useRef(null);
    const rlRef = useRef(null);
    const restartCountRef = useRef(0);
    const handlersRef = useRef(handlers);
    const configRef = useRef(config);
    // Keep refs up to date
    handlersRef.current = handlers;
    configRef.current = config;
    // -- Dispatch parsed event to appropriate handler --
    const dispatchEvent = useCallback((event) => {
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
    const send = useCallback((jsonLine) => {
        const proc = processRef.current;
        if (proc?.stdin?.writable) {
            proc.stdin.write(jsonLine);
        }
    }, []);
    // -- Process a single line from stdout --
    const processLine = useCallback((line) => {
        const trimmed = line.trim();
        if (!trimmed)
            return;
        try {
            const json = JSON.parse(trimmed);
            const result = EngineEventSchema.safeParse(json);
            if (result.success) {
                dispatchEvent(result.data);
            }
            else {
                handlersRef.current.onParseError?.(trimmed, result.error);
            }
        }
        catch (err) {
            handlersRef.current.onParseError?.(trimmed, err);
        }
    }, [dispatchEvent]);
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
        const rl = createInterface({ input: proc.stdout });
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
            }
            else {
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
                    if (!proc.killed)
                        proc.kill('SIGKILL');
                }, 2000);
            }
        };
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);
    // -- Public action methods --
    const sendPrompt = useCallback((text, opts) => {
        send(buildAction('submit_prompt', { text, ...opts }));
    }, [send]);
    const runInBackground = useCallback((text) => send(buildAction('run_in_background', { text })), [send]);
    const cancelAgent = useCallback((agentId) => send(buildAction('cancel_agent', { agentId })), [send]);
    const resolvePermission = useCallback((requestId, decision) => send(buildAction('permission_response', { requestId, allow: decision === 'allow' })), [send]);
    const resolveAskUser = useCallback((requestId, answer) => send(buildAction('ask_user_response', { requestId, response: answer })), [send]);
    const sendKbFeedback = useCallback((queryId, rating, comment, correction) => send(buildAction('knowledge_feedback', { queryId, rating, comment: comment ?? '', correction: correction ?? '' })), [send]);
    const changePermissionMode = useCallback((mode) => send(buildAction('update_permissions', { mode })), [send]);
    const toggleContextFile = useCallback((path, action) => send(buildAction('toggle_context_file', { path, action })), [send]);
    const changeRouting = useCallback((category, tier) => send(buildAction('change_routing', { category, tier })), [send]);
    const clearChat = useCallback(() => send(buildAction('clear_chat', {})), [send]);
    const slashCommand = useCallback((command) => send(buildAction('slash_command', { command })), [send]);
    const updateModel = useCallback((model) => send(buildAction('update_model', { model })), [send]);
    const moeDispatch = useCallback((commands) => send(buildAction('moe_dispatch', { commands })), [send]);
    const injectSkill = useCallback((command) => send(buildAction('inject_skill', { command })), [send]);
    const quit = useCallback(() => send(buildAction('quit', {})), [send]);
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
//# sourceMappingURL=use-engine.js.map