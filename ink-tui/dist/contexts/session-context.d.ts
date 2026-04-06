/**
 * SessionProvider — wraps the useSession hook in a context so that any
 * component in the tree (including the input box for /resume) can access
 * session persistence without prop drilling.
 *
 * Provider order in the tree (placed inside EngineProvider so it can read
 * chat messages and UI state):
 *
 *   ... > ChatProvider > EngineProvider > SessionProvider > DefaultLayout
 *
 * Auto-save triggers:
 *   - Every 60 s (handled inside useSession)
 *   - After each assistant message finishes (stream_end)
 *   - After each tool call completes
 *   - On unmount (cleanup / graceful exit)
 */
import React, { type ReactNode } from 'react';
import { type UseSessionReturn } from '../hooks/use-session.js';
export interface SessionProviderProps {
    children: ReactNode;
}
export declare function SessionProvider({ children }: SessionProviderProps): React.ReactElement;
/**
 * Access the session manager. Must be used within a <SessionProvider>.
 */
export declare function useSessionContext(): UseSessionReturn;
