import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
/**
 * StatusBar -- persistent single-line bar between chat and input.
 *
 * Matches Ratatui design:
 *
 *   Left side (when active):
 *     * Thinking... (4m 55s . down-arrow 5.0k tokens)
 *
 *   Right side (always):
 *     All keybinding hints in one line:
 *     Esc:input . Tab:section . j/k:nav . Ctrl+C:quit . Ctrl+B:bg . Ctrl+P:mode . F2:hide
 *
 * When idle/done the left side is hidden for a clean look.
 * The "done" checkmark auto-hides after 2 seconds.
 */
import { useState, useEffect, useRef } from 'react';
import { Box, Text } from 'ink';
import { useUIState } from '../contexts/ui-state-context.js';
import { useTheme } from '../contexts/theme-context.js';
import { OaSpinner } from './spinner.js';
// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------
/**
 * Format milliseconds into human-readable elapsed time.
 */
function formatElapsed(ms) {
    const totalSecs = Math.floor(ms / 1000);
    if (totalSecs < 60) {
        return `${totalSecs}s`;
    }
    const minutes = Math.floor(totalSecs / 60);
    const seconds = totalSecs % 60;
    if (minutes < 60) {
        return `${minutes}m ${String(seconds).padStart(2, '0')}s`;
    }
    const hours = Math.floor(minutes / 60);
    const remainMins = minutes % 60;
    return `${hours}h ${String(remainMins).padStart(2, '0')}m`;
}
/**
 * Format token count compactly.
 */
function formatTokens(tokens) {
    if (tokens < 1_000)
        return String(tokens);
    if (tokens < 1_000_000)
        return `${(tokens / 1_000).toFixed(1)}k`;
    return `${(tokens / 1_000_000).toFixed(1)}M`;
}
/**
 * Whether the phase represents active work.
 */
function isActivePhase(phase) {
    return (phase === 'thinking' ||
        phase === 'reading_file' ||
        phase === 'editing_file' ||
        phase === 'running_bash' ||
        phase === 'searching');
}
/**
 * Build keybinding hints string matching Ratatui.
 * Clean, minimal hints — only the essentials.
 */
function getHints(_mode, phase) {
    if (isActivePhase(phase)) {
        return 'Esc:scroll \u00B7 Ctrl+C:stop \u00B7 Ctrl+B:bg \u00B7 Ctrl+P:mode \u00B7 F2:sidebar';
    }
    return 'Esc:scroll \u00B7 Ctrl+C:quit \u00B7 Ctrl+B:bg \u00B7 Ctrl+P:mode \u00B7 F2:sidebar';
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export function StatusBar() {
    const { phase, phaseLabel, elapsedMs, tokensRemaining, mode, voiceRecording, sidebarAgents, } = useUIState();
    const { colors } = useTheme();
    // Track whether the "done" checkmark is still visible (auto-hides after 2s).
    const [showDone, setShowDone] = useState(false);
    const doneTimerRef = useRef(null);
    // Track "cooked" time: the duration of the last active phase.
    const [lastCookedMs, setLastCookedMs] = useState(null);
    const turnStartRef = useRef(null);
    const prevPhaseRef = useRef(phase);
    useEffect(() => {
        const prevPhase = prevPhaseRef.current;
        prevPhaseRef.current = phase;
        // When entering an active phase, record the start time.
        if (isActivePhase(phase) && !isActivePhase(prevPhase)) {
            turnStartRef.current = Date.now();
            setLastCookedMs(null);
        }
        // When leaving an active phase for done/idle, capture elapsed.
        if (!isActivePhase(phase) && isActivePhase(prevPhase) && turnStartRef.current != null) {
            setLastCookedMs(Date.now() - turnStartRef.current);
            turnStartRef.current = null;
        }
        // When entering a NEW active phase from idle/done, clear the cooked display.
        if (isActivePhase(phase) && (prevPhase === 'idle' || prevPhase === 'done')) {
            setLastCookedMs(null);
        }
    }, [phase]);
    useEffect(() => {
        if (phase === 'done') {
            setShowDone(true);
            doneTimerRef.current = setTimeout(() => {
                setShowDone(false);
            }, 2000);
        }
        else if (phase !== 'idle') {
            setShowDone(false);
        }
        return () => {
            if (doneTimerRef.current) {
                clearTimeout(doneTimerRef.current);
            }
        };
    }, [phase]);
    const active = isActivePhase(phase);
    const hints = getHints(mode, phase);
    // Count background agents currently running.
    const runningAgentCount = sidebarAgents.filter((a) => a.status === 'Running').length;
    // -- Left side --
    let leftContent = null;
    if (voiceRecording) {
        leftContent = (_jsx(Text, { color: colors.status.error, bold: true, children: '  \u{1F3A4} Recording...  [Space/Enter to stop \u00B7 Esc to cancel]' }));
    }
    else if (active) {
        const elapsed = formatElapsed(elapsedMs);
        const tokenPart = tokensRemaining != null
            ? ` \u00B7 \u2193 ${formatTokens(tokensRemaining)} tokens`
            : '';
        const agentPart = runningAgentCount > 0
            ? ` \u00B7 ${runningAgentCount} agent${runningAgentCount > 1 ? 's' : ''} running`
            : '';
        const statsStr = `(${elapsed}${tokenPart}${agentPart})`;
        leftContent = (_jsxs(Box, { children: [_jsxs(Text, { color: colors.text.accent, children: ['\u273B', " "] }), _jsx(OaSpinner, { active: true, label: phaseLabel || 'Working...' }), _jsxs(Text, { color: colors.text.secondary, children: [" ", statsStr] })] }));
    }
    else if ((phase === 'done' && showDone) || (phase === 'idle' && lastCookedMs != null)) {
        const cookedStr = formatElapsed(lastCookedMs ?? 0);
        const agentPart = runningAgentCount > 0
            ? ` \u00B7 ${runningAgentCount} agent${runningAgentCount > 1 ? 's' : ''} still running`
            : '';
        leftContent = (_jsxs(Box, { children: [_jsx(Text, { color: colors.status.done, children: '\u273B' }), _jsxs(Text, { color: colors.text.secondary, children: [" Cooked for ", cookedStr, agentPart] })] }));
    }
    else if (phase === 'error') {
        leftContent = (_jsxs(Text, { color: colors.status.error, bold: true, children: ['\u2717', " Error"] }));
    }
    // -- Right side: all hints --
    const rightContent = (_jsx(Text, { color: colors.text.secondary, children: hints }));
    return (_jsxs(Box, { width: "100%", justifyContent: "space-between", children: [_jsx(Box, { flexShrink: 1, children: leftContent }), _jsx(Box, { flexShrink: 0, children: rightContent })] }));
}
//# sourceMappingURL=status-bar.js.map