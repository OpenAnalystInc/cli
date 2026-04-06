/**
 * UIStateProvider — central UI state for the entire TUI.
 *
 * Split into two contexts for performance:
 * - UIStateContext — the state object (changes frequently)
 * - UIActionsContext — stable action functions (never change identity)
 *
 * Components that only need to dispatch actions (e.g. keybinding handlers)
 * subscribe to UIActionsContext and avoid re-rendering on every state tick.
 */
import React, { type ReactNode } from 'react';
import type { AgentPhase, PermissionMode } from '../types/messages.js';
export type AppMode = 'idle' | 'streaming' | 'scroll' | 'sidebar_focused' | 'voice_recording';
export type InputMode = 'ready' | 'agent_running' | 'plan_running' | 'streaming';
export interface PermissionDialogState {
    requestId: string;
    agentId: string;
    toolName: string;
    toolInput: string;
    requiredMode: PermissionMode;
    filePath?: string;
    description?: string;
    selectedButton: 'allow' | 'deny';
}
export interface AskUserDialogState {
    requestId: string;
    agentId: string;
    question: string;
    options?: string[];
    defaultValue?: string;
    allowFreeText: boolean;
    selectedIndex: number;
    typingMode: boolean;
    typedText: string;
}
export interface UIState {
    mode: AppMode;
    permissionMode: PermissionMode;
    sidebarVisible: boolean;
    sidebarFocused: boolean;
    scrollMode: boolean;
    autoScroll: boolean;
    scrollOffset: number;
    focusedMessageIndex: number;
    permissionDialog: PermissionDialogState | null;
    askUserDialog: AskUserDialogState | null;
    autocompleteVisible: boolean;
    autocompleteItems: string[];
    autocompleteIndex: number;
    voiceRecording: boolean;
    phase: AgentPhase;
    phaseLabel: string;
    elapsedMs: number;
    tokensRemaining: number | null;
    inputMode: InputMode;
    inputLabel: string;
    currentModel: string;
    currentBranch: string;
    activeAgent: string | null;
    contextFiles: string[];
    creditBalance: string | null;
    mcpServerCount: number;
    terminalWidth: number;
    terminalHeight: number;
    exitPending: boolean;
}
export interface UIActions {
    toggleSidebar(): void;
    focusSidebar(): void;
    hideSidebar(): void;
    enterScrollMode(): void;
    exitScrollMode(): void;
    cyclePermissionMode(): void;
    setPhase(phase: AgentPhase, label?: string): void;
    setElapsed(elapsedMs: number): void;
    setTokensRemaining(tokens: number | null): void;
    showPermissionDialog(state: PermissionDialogState): void;
    dismissPermissionDialog(): void;
    showAskUserDialog(state: AskUserDialogState): void;
    dismissAskUserDialog(): void;
    setAutoScroll(enabled: boolean): void;
    scrollBy(delta: number): void;
    scrollTo(offset: number): void;
    setFocusedMessage(index: number): void;
    addContextFile(path: string): void;
    removeContextFile(path: string): void;
    setContextFiles(files: string[]): void;
    setTerminalSize(width: number, height: number): void;
    setExitPending(pending: boolean): void;
    setMode(mode: AppMode): void;
    setInputMode(inputMode: InputMode, label?: string): void;
    setModelInfo(model: string): void;
    setBranch(branch: string): void;
    setActiveAgent(agentId: string | null): void;
    setCreditBalance(balance: string | null): void;
    setMcpServerCount(count: number): void;
    setAutocomplete(visible: boolean, items?: string[], index?: number): void;
    setVoiceRecording(recording: boolean): void;
    clearChat(): void;
}
export interface UIStateProviderProps {
    children: ReactNode;
}
export declare function UIStateProvider({ children }: UIStateProviderProps): React.ReactElement;
/**
 * Returns the current UI state. Re-renders on every state change.
 * Use `useUIActions()` if you only need to dispatch.
 */
export declare function useUIState(): UIState;
/**
 * Returns stable action dispatchers. Never causes re-renders.
 * Prefer this over `useUIState()` in event handlers and keybinding subscribers.
 */
export declare function useUIActions(): UIActions;
