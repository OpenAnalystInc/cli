/**
 * FileOutput — renders file output messages (image, audio, diagram, text).
 *
 * Visual structure:
 *   [IMG] Description of the generated image
 *         /path/to/output/file.png
 *
 * Type badges:
 *   IMG  — green (images)
 *   AUD  — blue (audio files)
 *   DGM  — cyan (diagrams)
 *   TXT  — dimmed (text files)
 *
 * All colors from useTheme() semantic tokens.
 */
import React from 'react';
import type { FileOutputType } from '../types/chat.js';
export interface FileOutputProps {
    fileType: FileOutputType;
    description: string;
    filePath: string;
    isFocused: boolean;
}
export declare function FileOutput({ fileType, description, filePath, isFocused, }: FileOutputProps): React.ReactElement;
