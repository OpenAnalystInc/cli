/**
 * ContextFileTags — renders @filename badges in the input box bottom border.
 *
 * Files are shown as cyan-on-dark badges. When the total width exceeds
 * the available space, remaining files are collapsed into a "+N more" indicator.
 *
 * All colors from useTheme() semantic tokens.
 */
import React from 'react';
export interface ContextFileTagsProps {
    /** Full file paths — only the filename portion is displayed. */
    files: string[];
    /** Maximum available width in columns for the tag row. */
    maxWidth: number;
}
export declare function ContextFileTags({ files, maxWidth, }: ContextFileTagsProps): React.ReactElement | null;
