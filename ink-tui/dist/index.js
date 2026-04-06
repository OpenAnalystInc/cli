import { jsx as _jsx } from "react/jsx-runtime";
import { render } from 'ink';
import { App } from './app.js';
// ---------------------------------------------------------------------------
// CLI argument parsing
// ---------------------------------------------------------------------------
const args = process.argv.slice(2);
const isMock = args.includes('--mock') || process.env['OA_MOCK'] === '1';
const enginePath = process.env['OA_ENGINE_PATH'] || 'openanalyst';
const engineConfig = {
    binaryPath: enginePath,
    mock: isMock,
    autoRestart: !isMock,
    maxRestartAttempts: 3,
};
// ---------------------------------------------------------------------------
// Render
// ---------------------------------------------------------------------------
const { unmount, waitUntilExit } = render(_jsx(App, { engineConfig: engineConfig }));
// ---------------------------------------------------------------------------
// Graceful shutdown
// ---------------------------------------------------------------------------
function cleanup() {
    unmount();
}
process.on('SIGTERM', cleanup);
process.on('SIGINT', cleanup);
// Handle uncaught errors gracefully
process.on('uncaughtException', (err) => {
    process.stderr.write(`\nFatal error: ${err.message}\n`);
    if (err.stack) {
        process.stderr.write(`${err.stack}\n`);
    }
    cleanup();
    process.exit(1);
});
process.on('unhandledRejection', (reason) => {
    process.stderr.write(`\nUnhandled rejection: ${String(reason)}\n`);
});
// Wait for the app to exit
waitUntilExit().then(() => {
    process.exit(0);
}).catch(() => {
    process.exit(1);
});
//# sourceMappingURL=index.js.map