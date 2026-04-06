/**
 * MarkdownRenderer — streaming-aware terminal markdown renderer.
 *
 * Parses markdown into blocks (paragraphs, headings, code blocks, lists,
 * blockquotes, tables) and renders them with Ink Text elements using
 * semantic theme colors.
 *
 * Streaming optimization: caches parsed blocks and only re-parses the
 * last block on each delta, avoiding full re-parse per frame.
 *
 * Uses lowlight (highlight.js) for code block syntax highlighting.
 */
import React from 'react';
export interface MarkdownRendererProps {
    /** The raw markdown string to render. */
    content: string;
    /** Whether the content is still being streamed. */
    isStreaming: boolean;
}
export declare const MarkdownRenderer: React.MemoExoticComponent<({ content, isStreaming, }: MarkdownRendererProps) => React.ReactElement>;
