/**
 * SidebarFiles — Touched files section for the sidebar panel.
 *
 * Displays files from the current session with action icons:
 *   - `○` read    (dimmed — sidebar.fileRead)
 *   - `●` edited  (yellow — sidebar.fileEdited)
 *   - `+` created (green — sidebar.fileCreated)
 *
 * Shows just the filename (not full path), truncated for 26-char width.
 * Enter toggles the file as a context file.
 */
import React from 'react';
import type { FileInfo } from '../types/messages.js';
import type { SemanticColors } from '../themes/semantic-tokens.js';
export interface SidebarFilesProps {
    files: readonly FileInfo[];
    selectedIndex: number;
    isFocused: boolean;
    colors: SemanticColors;
}
export declare function SidebarFiles({ files, selectedIndex, isFocused, colors, }: SidebarFilesProps): React.ReactElement;
