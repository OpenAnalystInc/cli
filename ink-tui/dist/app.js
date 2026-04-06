import { jsx as _jsx } from "react/jsx-runtime";
import { TerminalProvider } from './contexts/terminal-context.js';
import { ThemeProvider } from './contexts/theme-context.js';
import { KeypressProvider } from './contexts/keypress-context.js';
import { UIStateProvider } from './contexts/ui-state-context.js';
import { ChatProvider } from './contexts/chat-context.js';
import { PlaywrightMCPProvider } from './mcp/index.js';
import { EngineProvider } from './engine/index.js';
import { SessionProvider } from './contexts/session-context.js';
import { DefaultLayout } from './layouts/default-layout.js';
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function App({ engineConfig }) {
    const config = engineConfig ?? {
        binaryPath: process.env['OA_ENGINE_PATH'] || 'openanalyst',
        autoRestart: true,
        maxRestartAttempts: 3,
    };
    return (_jsx(TerminalProvider, { children: _jsx(ThemeProvider, { children: _jsx(KeypressProvider, { children: _jsx(UIStateProvider, { children: _jsx(ChatProvider, { children: _jsx(PlaywrightMCPProvider, { autoStart: true, children: _jsx(EngineProvider, { config: config, children: _jsx(SessionProvider, { children: _jsx(DefaultLayout, {}) }) }) }) }) }) }) }) }));
}
//# sourceMappingURL=app.js.map