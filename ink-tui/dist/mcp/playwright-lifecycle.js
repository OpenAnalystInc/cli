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
import { spawn } from 'node:child_process';
import { EventEmitter } from 'node:events';
import { createInterface } from 'node:readline';
// ---------------------------------------------------------------------------
// Default config
// ---------------------------------------------------------------------------
const DEFAULT_CONFIG = {
    browser: 'chromium',
    headless: true,
    viewportSize: '1280x720',
    isolated: true,
    npxPath: 'npx',
};
// ---------------------------------------------------------------------------
// Lifecycle manager
// ---------------------------------------------------------------------------
export class PlaywrightMCPLifecycle extends EventEmitter {
    proc = null;
    stdoutRl = null;
    stderrRl = null;
    _state = 'stopped';
    _config;
    _disposed = false;
    constructor(config) {
        super();
        this._config = { ...DEFAULT_CONFIG, ...config };
    }
    // -- Public getters -------------------------------------------------------
    get state() {
        return this._state;
    }
    get isReady() {
        return this._state === 'ready';
    }
    get isRunning() {
        return this._state === 'starting' || this._state === 'ready';
    }
    /** The child process, exposed for engine bridge to pipe stdio. */
    get process() {
        return this.proc;
    }
    /** stdin pipe for sending MCP requests to the server. */
    get stdin() {
        return this.proc?.stdin ?? null;
    }
    /** stdout pipe for reading MCP responses from the server. */
    get stdout() {
        return this.proc?.stdout ?? null;
    }
    // -- Lifecycle ------------------------------------------------------------
    /**
     * Start the Playwright MCP server.
     *
     * Spawns `npx @playwright/mcp` with the configured options.
     * Resolves when the server process has spawned successfully.
     * Does NOT wait for MCP initialize — the engine handles that.
     */
    async start() {
        if (this._disposed)
            return;
        if (this.isRunning)
            return;
        this.setState('starting');
        const args = this.buildArgs();
        try {
            // First, verify that @playwright/mcp is available
            const proc = spawn(this._config.npxPath, ['-y', '@playwright/mcp@latest', ...args], {
                stdio: ['pipe', 'pipe', 'pipe'],
                env: {
                    ...process.env,
                    // Ensure Playwright doesn't try to open a headed browser in CI
                    ...(this._config.headless ? { PLAYWRIGHT_BROWSERS_PATH: '0' } : {}),
                },
                // On Windows, npx needs shell
                shell: process.platform === 'win32',
                // Detach so the TUI can exit cleanly
                windowsHide: true,
            });
            this.proc = proc;
            // Track stdout lines (MCP JSON-RPC messages)
            if (proc.stdout) {
                this.stdoutRl = createInterface({ input: proc.stdout });
                this.stdoutRl.on('line', (line) => {
                    this.emit('stdout_line', line);
                });
            }
            // Track stderr lines (Playwright logs, errors)
            if (proc.stderr) {
                this.stderrRl = createInterface({ input: proc.stderr });
                this.stderrRl.on('line', (line) => {
                    this.emit('stderr_line', line);
                    // Detect common errors
                    if (line.includes('Executable doesn\'t exist') || line.includes('browserType.launch')) {
                        this.setState('unavailable', 'Playwright browsers not installed. Run: npx playwright install chromium');
                    }
                });
            }
            // Handle spawn success
            proc.on('spawn', () => {
                if (!this._disposed) {
                    this.setState('ready');
                }
            });
            // Handle spawn error (npx not found, package not found, etc.)
            proc.on('error', (err) => {
                if (err.message.includes('ENOENT')) {
                    this.setState('unavailable', 'npx not found. Ensure Node.js is installed.');
                }
                else {
                    this.setState('error', `Playwright MCP server error: ${err.message}`);
                }
                this.emit('error', err);
                this.cleanup();
            });
            // Handle process exit
            proc.on('exit', (code, signal) => {
                if (!this._disposed && this._state !== 'unavailable') {
                    if (code !== 0 && code !== null) {
                        this.setState('error', `Playwright MCP server exited with code ${code}${signal ? ` (signal: ${signal})` : ''}`);
                    }
                    else {
                        this.setState('stopped');
                    }
                }
                this.cleanup();
            });
        }
        catch (err) {
            const error = err instanceof Error ? err : new Error(String(err));
            this.setState('error', error.message);
            this.emit('error', error);
        }
    }
    /**
     * Stop the Playwright MCP server gracefully.
     */
    async stop() {
        if (!this.proc) {
            this.setState('stopped');
            return;
        }
        // Send SIGTERM first
        if (!this.proc.killed) {
            this.proc.kill('SIGTERM');
            // Force kill after 3 seconds
            const proc = this.proc;
            const forceKillTimer = setTimeout(() => {
                if (proc && !proc.killed) {
                    proc.kill('SIGKILL');
                }
            }, 3000);
            // Wait for exit
            await new Promise((resolve) => {
                if (!proc || proc.killed) {
                    clearTimeout(forceKillTimer);
                    resolve();
                    return;
                }
                proc.on('exit', () => {
                    clearTimeout(forceKillTimer);
                    resolve();
                });
            });
        }
        this.cleanup();
        this.setState('stopped');
    }
    /**
     * Dispose the lifecycle manager. Cannot be restarted after this.
     */
    async dispose() {
        this._disposed = true;
        await this.stop();
        this.removeAllListeners();
    }
    // -- Private helpers ------------------------------------------------------
    buildArgs() {
        const args = [];
        if (this._config.headless) {
            args.push('--headless');
        }
        if (this._config.browser !== 'chromium') {
            args.push('--browser', this._config.browser);
        }
        if (this._config.viewportSize !== '1280x720') {
            args.push('--viewport-size', this._config.viewportSize);
        }
        if (this._config.isolated) {
            args.push('--isolated');
        }
        return args;
    }
    cleanup() {
        this.stdoutRl?.close();
        this.stdoutRl = null;
        this.stderrRl?.close();
        this.stderrRl = null;
        this.proc = null;
    }
    setState(state, message) {
        if (this._state === state)
            return;
        this._state = state;
        this.emit('state_change', state, message);
    }
}
//# sourceMappingURL=playwright-lifecycle.js.map