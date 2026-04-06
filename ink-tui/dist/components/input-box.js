import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * InputBox -- minimal input area matching the Ratatui design.
 *
 * Visual structure:
 *
 *   [icon] Enter to send . Ctrl+P mode  [I] --------[No-Git]
 *   |  user types here...
 *   |                                           API credits
 *
 * Features:
 *   - Top line: prompt icon + hint text + vim indicator + horizontal rule + branch badge
 *   - The horizontal rule color changes by permission mode (blue/yellow/green/red)
 *   - Below: clean text input area with NO box border
 *   - Bottom-right: credit balance + MCP count
 *   - Multi-line text input with basic editing
 *   - Vim mode: normal / insert mode tracking
 *   - History navigation (Up/Down)
 *   - Enter to submit, dynamic height 3-8 lines
 *   - Disabled state during streaming/agent running
 *
 * All colors from useTheme() semantic tokens.
 * Keypress subscription at priority 3 (input mode).
 */
import { useState, useCallback, useRef } from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
import { useUIState, useUIActions } from '../contexts/ui-state-context.js';
import { useTerminal } from '../contexts/terminal-context.js';
import { useKeypress } from '../hooks/use-keypress.js';
import { useInputHistory } from '../hooks/use-input-history.js';
import { useChatActions } from '../contexts/chat-context.js';
import { useEngine } from '../engine/engine-context.js';
import { Command } from '../key/commands.js';
import { ModeBadge } from './mode-badge.js';
import { ContextFileTags } from './context-file-tags.js';
import { parseSlashCommand, formatHelpText } from '../utils/slash-commands.js';
import { useSessionContext } from '../contexts/session-context.js';
import { useVoice } from '../hooks/use-voice.js';
import { useCredits } from '../hooks/use-credits.js';
import { credentialManager, PROVIDER_CONFIG } from '../utils/credential-manager.js';
import { clearCreditCache } from '../utils/credit-checker.js';
import { providerPreferences } from '../utils/provider-preferences.js';
// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------
const INPUT_MIN_HEIGHT = 3;
const INPUT_MAX_HEIGHT = 8;
const BORDER_KEY_MAP = {
    default: 'default',
    plan: 'plan',
    acceptEdits: 'acceptEdits',
    danger: 'danger',
    streaming: 'streaming',
    agentRunning: 'agentRunning',
};
const PERMISSION_CONFIGS = {
    'prompt': { icon: '\u276F', label: 'Default', borderColorKey: 'default' },
    'read-only': { icon: '\u25C8', label: 'Plan', borderColorKey: 'plan' },
    'workspace-write': { icon: '\u270E', label: 'Accept Edits', borderColorKey: 'acceptEdits' },
    'danger-full-access': { icon: '\u26A1', label: 'Danger', borderColorKey: 'danger' },
};
function handleLoginCommand(args, chatActions, credits, engine) {
    // /login status -- show all saved credentials
    if (args === 'status') {
        const creds = credentialManager.listCredentials();
        if (creds.length === 0) {
            chatActions.pushSystem('No API keys configured.\n\nUse /login <provider> <api-key> to add one.', 'info');
            return;
        }
        const lines = ['Saved credentials:', ''];
        for (const cred of creds) {
            const config = PROVIDER_CONFIG[cred.provider];
            const masked = cred.apiKey.length > 8
                ? `${cred.apiKey.slice(0, 4)}...${cred.apiKey.slice(-4)}`
                : '****';
            lines.push(`  ${config?.displayName ?? cred.provider}: ${masked} (${cred.envVarName}, source: ${cred.source})`);
        }
        lines.push('');
        lines.push('Checking credits...');
        chatActions.pushSystem(lines.join('\n'), 'info');
        // Fetch credits asynchronously and display
        void credentialManager.fetchAllCredits().then((creditMap) => {
            const creditLines = ['Provider credits:', ''];
            for (const [provider, balance] of Object.entries(creditMap)) {
                const config = PROVIDER_CONFIG[provider];
                creditLines.push(`  ${config?.displayName ?? provider}: ${balance}`);
            }
            chatActions.pushSystem(creditLines.join('\n'), 'info');
        });
        return;
    }
    // /login gemini oauth -- delegate to Rust engine for browser-based OAuth
    if (args === 'gemini oauth') {
        engine.bridge.slashCommand('/login gemini oauth');
        return;
    }
    // /login (no args) -- show usage
    if (!args) {
        const lines = [
            'Available providers:',
            '',
            '  1. openai      (OPENAI_API_KEY)     -- sk-...',
            '  2. anthropic   (ANTHROPIC_API_KEY)   -- sk-ant-...',
            '  3. gemini      (GEMINI_API_KEY)      -- AIza... or OAuth',
            '  4. xai         (XAI_API_KEY)         -- xai-...',
            '  5. openrouter  (OPENROUTER_API_KEY)  -- sk-or-...',
            '  6. bedrock     (BEDROCK_API_KEY)',
            '  7. stability   (STABILITY_API_KEY)',
            '  8. openanalyst (OPENANALYST_AUTH_TOKEN)',
            '',
            'Usage:',
            '  /login <provider> <api-key>  Save an API key',
            '  /login <api-key>             Auto-detect provider from key prefix',
            '  /login gemini oauth          Browser-based OAuth for Google Gemini',
            '  /login status                Show all saved credentials',
            '',
            'Example:',
            '  /login openai sk-abc123...',
            '  /login sk-ant-abc123...       (auto-detects Anthropic)',
        ];
        chatActions.pushSystem(lines.join('\n'), 'info');
        return;
    }
    // /login <provider> <key> or /login <key>
    const parts = args.split(/\s+/);
    let provider = null;
    let apiKey;
    if (parts.length >= 2) {
        // Explicit provider + key
        const providerArg = parts[0].toLowerCase();
        apiKey = parts.slice(1).join('');
        if (PROVIDER_CONFIG[providerArg]) {
            provider = providerArg;
        }
        else {
            chatActions.pushSystem(`Unknown provider: "${providerArg}". Use /login to see available providers.`, 'error');
            return;
        }
    }
    else {
        // Auto-detect from key prefix
        apiKey = parts[0];
        provider = credentialManager.detectProvider(apiKey);
        if (!provider) {
            chatActions.pushSystem('Could not auto-detect provider from key prefix.\nUse /login <provider> <key> instead.', 'error');
            return;
        }
    }
    const config = PROVIDER_CONFIG[provider];
    // Save the key and set as default provider
    void credentialManager.saveApiKey(provider, apiKey).then(() => {
        clearCreditCache();
        credits.refresh();
        // Set this provider as the user's default
        providerPreferences.setDefaultProvider(provider);
        providerPreferences.invalidateCache();
        const masked = apiKey.length > 8
            ? `${apiKey.slice(0, 4)}...${apiKey.slice(-4)}`
            : '****';
        chatActions.pushSystem(`${config.displayName} API key saved to global .env, SQLite, and credentials.json.\n` +
            `  Key: ${masked}\n` +
            `  Env: ${config.envVar}\n\n` +
            `\u2713 ${config.displayName} set as your default AI provider\n\n` +
            `Checking credits...`, 'info');
        // Fetch and display credits
        void credentialManager.fetchCredits(provider).then((creditStr) => {
            chatActions.pushSystem(`${config.displayName}: ${creditStr}`, 'info');
        });
    }).catch((err) => {
        chatActions.pushSystem(`Failed to save API key: ${err instanceof Error ? err.message : String(err)}`, 'error');
    });
}
function handleLogoutCommand(args, chatActions, credits) {
    if (!args) {
        chatActions.pushSystem('Usage:\n  /logout <provider>  Remove a specific provider\n  /logout all         Remove all credentials (keeps Gemini OAuth)', 'info');
        return;
    }
    if (args === 'all') {
        void credentialManager.removeAll().then(() => {
            clearCreditCache();
            credits.refresh();
            chatActions.pushSystem('All credentials removed (Gemini OAuth token preserved).\nUse /login to add new credentials.', 'info');
        });
        return;
    }
    const provider = args.toLowerCase();
    if (!PROVIDER_CONFIG[provider]) {
        chatActions.pushSystem(`Unknown provider: "${args}". Use /login status to see configured providers.`, 'error');
        return;
    }
    void credentialManager.removeCredential(provider).then(() => {
        clearCreditCache();
        credits.refresh();
        chatActions.pushSystem(`${PROVIDER_CONFIG[provider].displayName} credentials removed from all locations.`, 'info');
    });
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function InputBox({ onSubmit }) {
    const { colors } = useTheme();
    const ui = useUIState();
    const actions = useUIActions();
    const terminal = useTerminal();
    const history = useInputHistory();
    const chatActions = useChatActions();
    const engine = useEngine();
    const session = useSessionContext();
    const voice = useVoice();
    const credits = useCredits();
    // Text state
    const [text, setText] = useState('');
    const [cursorPos, setCursorPos] = useState(0);
    const [vimMode, setVimMode] = useState('insert');
    // Track whether we are actively navigating history
    const isNavigatingHistory = useRef(false);
    // Derived state
    const isDisabled = ui.inputMode === 'agent_running' || ui.inputMode === 'streaming' || ui.inputMode === 'plan_running';
    const permConfig = PERMISSION_CONFIGS[ui.permissionMode];
    // Determine line color (the horizontal rule and prompt icon)
    const lineColor = (() => {
        if (ui.inputMode === 'streaming')
            return colors.border.input.streaming;
        if (ui.inputMode === 'agent_running')
            return colors.border.input.agentRunning;
        if (ui.inputMode === 'plan_running')
            return colors.border.input.plan;
        return colors.border.input[permConfig.borderColorKey];
    })();
    // Dynamic height based on content lines
    const lineCount = text.split('\n').length;
    const dynamicHeight = Math.min(Math.max(lineCount + 2, INPUT_MIN_HEIGHT), INPUT_MAX_HEIGHT);
    // Submit handler -- routes slash commands locally or to engine
    const handleSubmit = useCallback(() => {
        const trimmed = text.trim();
        if (!trimmed || isDisabled)
            return;
        history.push(trimmed);
        // Check for slash commands
        if (trimmed.startsWith('/')) {
            const parsed = parseSlashCommand(trimmed);
            if (parsed) {
                // Show the user input in chat
                chatActions.pushUser(trimmed);
                if (parsed.handler === 'local') {
                    // Handle locally
                    switch (parsed.name) {
                        case 'help':
                            chatActions.pushSystem(formatHelpText(), 'info');
                            break;
                        case 'clear':
                            engine.clearChat();
                            break;
                        case 'login': {
                            const args = parsed.args.trim();
                            handleLoginCommand(args, chatActions, credits, engine);
                            break;
                        }
                        case 'logout': {
                            const args = parsed.args.trim();
                            handleLogoutCommand(args, chatActions, credits);
                            break;
                        }
                        case 'models': {
                            const output = providerPreferences.formatModelsOutput(ui.currentModel);
                            chatActions.pushSystem(output, 'info');
                            break;
                        }
                        case 'resume': {
                            const args = parsed.args.trim();
                            if (args === 'list') {
                                // Show recent sessions
                                const sessions = session.listSessions();
                                if (sessions.length === 0) {
                                    chatActions.pushSystem('No saved sessions found.', 'info');
                                }
                                else {
                                    const lines = ['Recent sessions:', ''];
                                    for (const s of sessions.slice(0, 10)) {
                                        const date = new Date(s.updatedAt).toLocaleString();
                                        lines.push(`  ${s.id}`);
                                        lines.push(`    ${s.summary.slice(0, 80)} (${s.messageCount} messages, ${date})`);
                                        lines.push('');
                                    }
                                    lines.push('Use /resume <session-id> to load a specific session.');
                                    chatActions.pushSystem(lines.join('\n'), 'info');
                                }
                            }
                            else if (args.length > 0) {
                                // Load specific session
                                const loaded = session.loadSession(args);
                                if (loaded) {
                                    chatActions.pushSystem(`Session resumed: ${args}`, 'info');
                                }
                                else {
                                    chatActions.pushSystem(`Session not found: ${args}`, 'error');
                                }
                            }
                            else {
                                // Resume most recent
                                const resumed = session.resumeLatest();
                                if (resumed) {
                                    chatActions.pushSystem('Resumed most recent session.', 'info');
                                }
                                else {
                                    chatActions.pushSystem('No saved sessions found. Start chatting to create one.', 'info');
                                }
                            }
                            break;
                        }
                    }
                }
                else {
                    // Send to engine via bridge
                    engine.bridge.slashCommand(trimmed);
                }
                setText('');
                setCursorPos(0);
                setVimMode('insert');
                isNavigatingHistory.current = false;
                return;
            }
            // Unknown slash command -- still submit as regular prompt
        }
        onSubmit?.(trimmed);
        setText('');
        setCursorPos(0);
        setVimMode('insert');
        isNavigatingHistory.current = false;
    }, [text, isDisabled, history, onSubmit, chatActions, engine]);
    // Insert a character at cursor position
    const insertChar = useCallback((char) => {
        setText((prev) => {
            const next = prev.slice(0, cursorPos) + char + prev.slice(cursorPos);
            setCursorPos((c) => c + char.length);
            return next;
        });
        isNavigatingHistory.current = false;
    }, [cursorPos]);
    // Backspace
    const handleBackspace = useCallback(() => {
        if (cursorPos === 0) {
            // If input is empty and there are context files, remove the last one
            if (text.length === 0 && ui.contextFiles.length > 0) {
                const lastFile = ui.contextFiles[ui.contextFiles.length - 1];
                actions.removeContextFile(lastFile);
            }
            return;
        }
        setText((prev) => prev.slice(0, cursorPos - 1) + prev.slice(cursorPos));
        setCursorPos((c) => Math.max(0, c - 1));
        isNavigatingHistory.current = false;
    }, [cursorPos, text, ui.contextFiles, actions]);
    // Delete key
    const handleDelete = useCallback(() => {
        if (cursorPos >= text.length)
            return;
        setText((prev) => prev.slice(0, cursorPos) + prev.slice(cursorPos + 1));
    }, [cursorPos, text.length]);
    // Cursor movement
    const moveLeft = useCallback(() => setCursorPos((c) => Math.max(0, c - 1)), []);
    const moveRight = useCallback(() => setCursorPos((c) => Math.min(text.length, c + 1)), [text.length]);
    const moveHome = useCallback(() => {
        // Move to start of current line
        const lineStart = text.lastIndexOf('\n', cursorPos - 1) + 1;
        setCursorPos(lineStart);
    }, [text, cursorPos]);
    const moveEnd = useCallback(() => {
        // Move to end of current line
        let lineEnd = text.indexOf('\n', cursorPos);
        if (lineEnd === -1)
            lineEnd = text.length;
        setCursorPos(lineEnd);
    }, [text, cursorPos]);
    // History navigation
    const historyUp = useCallback(() => {
        const entry = history.goUp(text);
        if (entry !== null) {
            setText(entry);
            setCursorPos(entry.length);
            isNavigatingHistory.current = true;
        }
    }, [history, text]);
    const historyDown = useCallback(() => {
        const entry = history.goDown();
        if (entry !== null) {
            setText(entry);
            setCursorPos(entry.length);
            isNavigatingHistory.current = true;
        }
        else {
            isNavigatingHistory.current = false;
        }
    }, [history]);
    // Keypress handler at priority 3 (input mode)
    useKeypress(useCallback((input, key, command) => {
        // Don't consume events when disabled (let them pass to global handlers)
        if (isDisabled)
            return false;
        // --- Vim normal mode handling ---
        if (vimMode === 'normal') {
            // Mode transitions
            if (input === 'i') {
                setVimMode('insert');
                return true;
            }
            if (input === 'I') {
                moveHome();
                setVimMode('insert');
                return true;
            }
            if (input === 'a') {
                moveRight();
                setVimMode('insert');
                return true;
            }
            if (input === 'A') {
                moveEnd();
                setVimMode('insert');
                return true;
            }
            if (input === 'o') {
                // Insert newline below and enter insert mode
                moveEnd();
                insertChar('\n');
                setVimMode('insert');
                return true;
            }
            // Navigation
            if (input === 'h' || key.leftArrow) {
                moveLeft();
                return true;
            }
            if (input === 'l' || key.rightArrow) {
                moveRight();
                return true;
            }
            if (input === 'j' || key.downArrow) {
                historyDown();
                return true;
            }
            if (input === 'k' || key.upArrow) {
                historyUp();
                return true;
            }
            if (input === '0') {
                moveHome();
                return true;
            }
            if (input === '$') {
                moveEnd();
                return true;
            }
            // Editing in normal mode
            if (input === 'x') {
                handleDelete();
                return true;
            }
            if (input === 'X') {
                handleBackspace();
                return true;
            }
            // Word-level delete (dw) -- simplified: delete to next space
            if (input === 'd') {
                // For simplicity, 'dd' clears the line
                // TODO: Full vim motion support
                return true;
            }
            // Don't consume other keys in normal mode -- let globals handle them
            return false;
        }
        // --- Insert mode handling ---
        // Escape -> enter normal mode
        if (key.escape) {
            // If the input is empty and not navigating history, let escape
            // propagate to enter scroll mode (handled by command matching)
            if (text.length === 0 && command === Command.ENTER_SCROLL_MODE) {
                setVimMode('normal');
                return false; // Let the scroll mode handler pick it up
            }
            setVimMode('normal');
            return true;
        }
        // Command-based handling
        if (command === Command.SUBMIT) {
            handleSubmit();
            return true;
        }
        if (command === Command.HISTORY_UP) {
            // Only navigate history if text is single-line
            if (!text.includes('\n')) {
                historyUp();
                return true;
            }
            return false;
        }
        if (command === Command.HISTORY_DOWN) {
            if (!text.includes('\n')) {
                historyDown();
                return true;
            }
            return false;
        }
        if (command === Command.REMOVE_LAST_CONTEXT_FILE) {
            if (text.length === 0 && ui.contextFiles.length > 0) {
                const lastFile = ui.contextFiles[ui.contextFiles.length - 1];
                actions.removeContextFile(lastFile);
                return true;
            }
        }
        // Ctrl+B: run current input in background
        if (command === Command.RUN_IN_BACKGROUND) {
            if (text.trim().length > 0) {
                const trimmed = text.trim();
                history.push(trimmed);
                chatActions.pushUser(trimmed);
                engine.bridge.sendAction('run_in_background', { text: trimmed });
                chatActions.pushSystem('Running in background...', 'info');
                setText('');
                setCursorPos(0);
                setVimMode('insert');
                isNavigatingHistory.current = false;
                return true;
            }
            // Empty input -- let global handler deal with it
            return false;
        }
        // Voice recording: Space on empty input starts recording
        if (command === Command.START_VOICE_RECORDING) {
            if (text.length === 0 && !voice.isRecording) {
                voice.startRecording();
                return true;
            }
        }
        // Special keys
        if (key.backspace || key.delete) {
            if (key.backspace)
                handleBackspace();
            else
                handleDelete();
            return true;
        }
        if (key.leftArrow) {
            moveLeft();
            return true;
        }
        if (key.rightArrow) {
            moveRight();
            return true;
        }
        if (key.upArrow) {
            if (!text.includes('\n')) {
                historyUp();
                return true;
            }
            // TODO: multi-line cursor up
            return true;
        }
        if (key.downArrow) {
            if (!text.includes('\n')) {
                historyDown();
                return true;
            }
            // TODO: multi-line cursor down
            return true;
        }
        // Return with shift -> newline
        if (key.return && key.shift) {
            insertChar('\n');
            return true;
        }
        // Regular Return -> submit
        if (key.return) {
            handleSubmit();
            return true;
        }
        // Tab -> insert spaces (or future autocomplete trigger)
        if (key.tab) {
            insertChar('  ');
            return true;
        }
        // Printable characters
        if (input && !key.ctrl && !key.meta) {
            insertChar(input);
            return true;
        }
        // Ctrl+U -> clear line
        if (key.ctrl && input === 'u') {
            setText('');
            setCursorPos(0);
            return true;
        }
        // Ctrl+A -> move to start
        if (key.ctrl && input === 'a') {
            setCursorPos(0);
            return true;
        }
        // Ctrl+E -> move to end
        if (key.ctrl && input === 'e') {
            setCursorPos(text.length);
            return true;
        }
        // Ctrl+W -> delete word backward
        if (key.ctrl && input === 'w') {
            const before = text.slice(0, cursorPos);
            const trimmed = before.replace(/\s+$/, '');
            const wordStart = Math.max(0, trimmed.lastIndexOf(' ') + 1);
            setText(text.slice(0, wordStart) + text.slice(cursorPos));
            setCursorPos(wordStart);
            return true;
        }
        return false;
    }, [
        isDisabled, vimMode, text, cursorPos, ui.contextFiles,
        handleSubmit, insertChar, handleBackspace, handleDelete,
        moveLeft, moveRight, moveHome, moveEnd,
        historyUp, historyDown, actions, voice,
    ]), { isActive: !isDisabled, priority: 3 });
    // Voice recording keypress handler (priority 7 -- intercepts during recording)
    useKeypress(useCallback((input, key, command) => {
        if (command === Command.VOICE_STOP) {
            if (key.escape) {
                // Esc cancels without transcribing
                voice.cancelRecording();
            }
            else {
                // Space or Enter: stop + transcribe
                voice.stopRecording().then((transcript) => {
                    if (transcript) {
                        setText(transcript);
                        setCursorPos(transcript.length);
                        setVimMode('insert');
                    }
                });
            }
            return true;
        }
        // Block all other keys during voice recording
        return true;
    }, [voice]), { isActive: voice.isRecording, priority: 7 });
    // -------------------------------------------------------------------------
    // Render helpers
    // -------------------------------------------------------------------------
    // Get the visible text with cursor indicator
    const renderTextWithCursor = () => {
        if (isDisabled) {
            const disabledLabel = ui.inputMode === 'streaming'
                ? 'Responding...'
                : ui.inputMode === 'agent_running'
                    ? `${ui.inputLabel || 'Agent running'}...`
                    : `${ui.inputLabel || 'Plan running'}...`;
            return [
                _jsx(Text, { color: colors.text.secondary, dimColor: true, children: `  ${disabledLabel}` }, "disabled"),
            ];
        }
        if (text.length === 0) {
            // Placeholder
            const placeholder = vimMode === 'normal'
                ? "  Press 'i' for INSERT mode"
                : '  Type your message or @path/to/file';
            return [
                _jsx(Text, { color: colors.text.secondary, dimColor: true, children: placeholder }, "placeholder"),
            ];
        }
        // Render text with a visible cursor
        const lines = text.split('\n');
        const elements = [];
        let charIndex = 0;
        for (let lineIdx = 0; lineIdx < lines.length; lineIdx++) {
            const line = lines[lineIdx];
            const lineStart = charIndex;
            const lineEnd = lineStart + line.length;
            if (cursorPos >= lineStart && cursorPos <= lineEnd) {
                // Cursor is on this line
                const localCursor = cursorPos - lineStart;
                const before = line.slice(0, localCursor);
                const cursorChar = line[localCursor] ?? ' ';
                const after = line.slice(localCursor + 1);
                elements.push(_jsxs(Text, { children: [_jsxs(Text, { color: colors.text.primary, children: ['  ', before] }), _jsx(Text, { color: vimMode === 'normal' ? '#000000' : colors.text.primary, backgroundColor: vimMode === 'normal' ? colors.text.primary : colors.text.accent, children: cursorChar }), _jsx(Text, { color: colors.text.primary, children: after })] }, `line-${lineIdx}`));
            }
            else {
                elements.push(_jsxs(Text, { color: colors.text.primary, children: ['  ', line] }, `line-${lineIdx}`));
            }
            // +1 for the newline character between lines
            charIndex = lineEnd + 1;
        }
        return elements;
    };
    // -------------------------------------------------------------------------
    // Top prompt line (the main visual change from the old design)
    // -------------------------------------------------------------------------
    const renderPromptLine = () => {
        // Build left portion: icon + hint + vim mode
        const leftParts = [];
        if (isDisabled) {
            const icon = ui.inputMode === 'streaming' ? '\u2819' : '\u25CF';
            const hintColor = ui.inputMode === 'streaming'
                ? colors.border.input.streaming
                : colors.border.input.agentRunning;
            const hintText = ui.inputMode === 'streaming'
                ? 'Responding... \u00B7 Ctrl+C to cancel'
                : `${ui.inputLabel || 'Running'} \u00B7 Ctrl+C to cancel`;
            leftParts.push(_jsxs(Text, { color: hintColor, bold: true, children: [icon, " "] }, "disabled-icon"));
            leftParts.push(_jsxs(Text, { color: colors.text.secondary, children: [hintText, " "] }, "disabled-hint"));
        }
        else {
            leftParts.push(_jsxs(Text, { color: lineColor, bold: true, children: [permConfig.icon, " "] }, "icon"));
            leftParts.push(_jsxs(Text, { color: colors.text.secondary, children: ["Enter to send ", '\u00B7', " Ctrl+P mode "] }, "hint"));
            if (vimMode === 'normal') {
                leftParts.push(_jsx(Text, { color: colors.status.warning, bold: true, children: "[N] " }, "vim"));
            }
            else {
                leftParts.push(_jsx(Text, { color: colors.status.done, bold: true, children: "[I] " }, "vim"));
            }
        }
        // Build right portion: badges + branch badge
        const rightParts = [];
        // Permission mode badge (only when not default)
        if (!isDisabled && ui.permissionMode !== 'prompt') {
            rightParts.push(_jsx(ModeBadge, { label: `${permConfig.icon} ${permConfig.label}`, bgColor: lineColor, textColor: "#000000" }, "mode"));
            rightParts.push(_jsx(Text, { children: " " }, "mode-sep"));
        }
        // Active agent badge
        if (ui.activeAgent) {
            rightParts.push(_jsx(ModeBadge, { label: ui.activeAgent, bgColor: colors.background.badge.agent, textColor: "#000000" }, "agent"));
            rightParts.push(_jsx(Text, { children: " " }, "agent-sep"));
        }
        // Model badge
        if (ui.currentModel) {
            rightParts.push(_jsx(ModeBadge, { label: ui.currentModel, bgColor: colors.background.badge.model, textColor: colors.text.primary }, "model"));
            rightParts.push(_jsx(Text, { children: " " }, "model-sep"));
        }
        // Branch badge at the end of the horizontal line
        const branchText = ui.currentBranch || 'No-Git';
        // Calculate how much space the left text + right badges take
        // We'll let Ink handle the layout via flexbox
        rightParts.push(_jsxs(Text, { color: lineColor, children: ["[", branchText, "]"] }, "branch"));
        return (_jsxs(Box, { justifyContent: "space-between", width: terminal.width, children: [_jsx(Box, { flexShrink: 1, children: leftParts }), _jsx(Box, { flexShrink: 0, children: rightParts })] }));
    };
    // -------------------------------------------------------------------------
    // Horizontal rule between prompt line and text area
    // -------------------------------------------------------------------------
    const renderHorizontalRule = () => {
        const ruleWidth = Math.max(0, terminal.width);
        return (_jsx(Text, { color: lineColor, children: '\u2500'.repeat(ruleWidth) }));
    };
    // -------------------------------------------------------------------------
    // Bottom status line
    // -------------------------------------------------------------------------
    const renderBottomLine = () => {
        const leftParts = [];
        // Context file tags
        if (ui.contextFiles.length > 0) {
            leftParts.push(_jsx(ContextFileTags, { files: ui.contextFiles, maxWidth: Math.floor(terminal.width * 0.6) }, "ctx"));
        }
        const rightParts = [];
        // Real credit balance from API provider
        const creditLabel = credits.provider !== 'unknown'
            ? `${credits.provider}: ${credits.balance}`
            : credits.balance;
        rightParts.push(_jsx(Text, { color: credits.loading ? colors.text.secondary : colors.status.done, dimColor: credits.loading, children: creditLabel }, "credits"));
        if (ui.mcpServerCount > 0) {
            rightParts.push(_jsxs(Text, { color: colors.status.done, dimColor: true, children: [rightParts.length > 0 ? ' ' : '', "MCP:", ui.mcpServerCount] }, "mcp"));
        }
        return (_jsxs(Box, { justifyContent: "space-between", width: terminal.width, children: [_jsx(Box, { flexShrink: 1, children: leftParts }), _jsx(Box, { flexShrink: 0, children: rightParts })] }));
    };
    // -------------------------------------------------------------------------
    // Main render -- NO BORDER BOX, just stacked lines
    // -------------------------------------------------------------------------
    return (_jsxs(Box, { flexDirection: "column", width: terminal.width, minHeight: INPUT_MIN_HEIGHT, height: dynamicHeight, children: [renderPromptLine(), renderHorizontalRule(), _jsx(Box, { flexDirection: "column", flexGrow: 1, paddingX: 0, children: renderTextWithCursor() }), renderBottomLine()] }));
}
//# sourceMappingURL=input-box.js.map