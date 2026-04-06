/**
 * SystemMessage — renders info, warning, and error notices in the chat.
 *
 * - Gray bullet + dim text for info
 * - Yellow bullet + yellow text for warnings
 * - Red bullet + red text for errors
 *
 * Agent lifecycle noise (spawned/completed) is filtered unless it's an error.
 */
import React from 'react';
import type { SystemLevel } from '../types/messages.js';
export interface SystemMessageProps {
    text: string;
    level: SystemLevel;
    isFocused?: boolean;
}
export declare function isLifecycleNoise(text: string, level: SystemLevel): boolean;
export declare const SystemMessage: React.MemoExoticComponent<({ text, level, isFocused, }: SystemMessageProps) => React.ReactElement | null>;
