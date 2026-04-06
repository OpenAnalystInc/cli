/**
 * PlaywrightMCPLifecycle — manages the @playwright/mcp server as a child process.
 *
 * The official @playwright/mcp package from Microsoft provides a full MCP server
 * that communicates via stdio (JSON-RPC 2.0). This class:
 *
 *   1. Spawns `npx @playwright/mcp --headless` as a child process on start()
 *   2. Passes through stdin/stdout for MCP protocol communication
 *   3. Tracks readiness via the MCP initialize handshake
 *   4. Cleanly shuts down the process on stop()
 *   5. Handles missing Playwright gracefully (no crash, just a warning)
 *
 * The Rust engine connects to this server's stdio pipes to use browser tools.
 */
import { type ChildProcess } from 'node:child_process';
import { EventEmitter } from 'node:events';
export type PlaywrightMCPState = 'stopped' | 'starting' | 'ready' | 'error' | 'unavailable';
export interface PlaywrightMCPConfig {
    /** Browser to launch. Default: 'chromium' */
    browser?: 'chromium' | 'firefox' | 'webkit' | 'msedge';
    /** Run headless (no visible window). Default: true */
    headless?: boolean;
    /** Viewport width x height. Default: '1280x720' */
    viewportSize?: string;
    /** Use isolated in-memory browser profiles. Default: true */
    isolated?: boolean;
    /** Path to npx binary. Default: 'npx' */
    npxPath?: string;
}
export interface PlaywrightMCPEvents {
    state_change: [state: PlaywrightMCPState, message?: string];
    stdout_line: [line: string];
    stderr_line: [line: string];
    error: [error: Error];
}
export declare class PlaywrightMCPLifecycle extends EventEmitter {
    private proc;
    private stdoutRl;
    private stderrRl;
    private _state;
    private _config;
    private _disposed;
    constructor(config?: PlaywrightMCPConfig);
    get state(): PlaywrightMCPState;
    get isReady(): boolean;
    get isRunning(): boolean;
    /** The child process, exposed for engine bridge to pipe stdio. */
    get process(): ChildProcess | null;
    /** stdin pipe for sending MCP requests to the server. */
    get stdin(): NodeJS.WritableStream | null;
    /** stdout pipe for reading MCP responses from the server. */
    get stdout(): NodeJS.ReadableStream | null;
    /**
     * Start the Playwright MCP server.
     *
     * Spawns `npx @playwright/mcp` with the configured options.
     * Resolves when the server process has spawned successfully.
     * Does NOT wait for MCP initialize — the engine handles that.
     */
    start(): Promise<void>;
    /**
     * Stop the Playwright MCP server gracefully.
     */
    stop(): Promise<void>;
    /**
     * Dispose the lifecycle manager. Cannot be restarted after this.
     */
    dispose(): Promise<void>;
    private buildArgs;
    private cleanup;
    private setState;
}
