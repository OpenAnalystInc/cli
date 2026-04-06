/**
 * AssistantMessage — renders an LLM response with markdown formatting.
 *
 * - 2-space left indent
 * - Full markdown rendering with syntax highlighting
 * - During streaming: shows content accumulated so far + blinking cursor
 */
import React from 'react';
export interface AssistantMessageProps {
    /** Accumulated markdown content. */
    content: string;
    /** True while the LLM is still generating tokens. */
    streaming: boolean;
    /** Whether this message is currently focused in scroll mode. */
    isFocused?: boolean;
}
export declare const AssistantMessage: React.MemoExoticComponent<({ content, streaming, isFocused, }: AssistantMessageProps) => React.ReactElement>;
