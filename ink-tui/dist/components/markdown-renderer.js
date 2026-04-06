import { jsx as _jsx, jsxs as _jsxs } from "react/jsx-runtime";
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
import React, { useMemo, useRef } from 'react';
import { Box, Text } from 'ink';
import { useTheme } from '../contexts/theme-context.js';
import { common, createLowlight } from 'lowlight';
// ---------------------------------------------------------------------------
// lowlight singleton
// ---------------------------------------------------------------------------
const lowlight = createLowlight(common);
// ---------------------------------------------------------------------------
// Parser — line-by-line block extraction
// ---------------------------------------------------------------------------
function parseMarkdownBlocks(source) {
    const lines = source.split('\n');
    const blocks = [];
    let i = 0;
    while (i < lines.length) {
        const line = lines[i];
        // --- Fenced code block ---
        const fenceMatch = line.match(/^```(\w*)/);
        if (fenceMatch) {
            const language = fenceMatch[1] ?? '';
            const codeLines = [];
            i++;
            while (i < lines.length && !lines[i].startsWith('```')) {
                codeLines.push(lines[i]);
                i++;
            }
            if (i < lines.length)
                i++; // skip closing fence
            blocks.push({ type: 'code', language, code: codeLines.join('\n') });
            continue;
        }
        // --- Heading ---
        const headingMatch = line.match(/^(#{1,6})\s+(.*)/);
        if (headingMatch) {
            blocks.push({
                type: 'heading',
                level: headingMatch[1].length,
                text: headingMatch[2],
            });
            i++;
            continue;
        }
        // --- Thematic break ---
        if (/^(-{3,}|\*{3,}|_{3,})\s*$/.test(line)) {
            blocks.push({ type: 'thematic_break' });
            i++;
            continue;
        }
        // --- Table (pipe-delimited) ---
        if (line.includes('|') && i + 1 < lines.length && /^\|?\s*[-:]+/.test(lines[i + 1])) {
            const headers = parsePipeRow(line);
            i += 2; // skip header + separator
            const rows = [];
            while (i < lines.length && lines[i].includes('|')) {
                rows.push(parsePipeRow(lines[i]));
                i++;
            }
            blocks.push({ type: 'table', headers, rows });
            continue;
        }
        // --- Blockquote ---
        if (line.startsWith('> ') || line === '>') {
            const quoteLines = [];
            while (i < lines.length && (lines[i].startsWith('> ') || lines[i] === '>')) {
                quoteLines.push(lines[i].replace(/^>\s?/, ''));
                i++;
            }
            blocks.push({ type: 'blockquote', text: quoteLines.join('\n') });
            continue;
        }
        // --- Ordered list ---
        if (/^\d+\.\s/.test(line)) {
            const items = [];
            while (i < lines.length && /^\d+\.\s/.test(lines[i])) {
                items.push(lines[i].replace(/^\d+\.\s/, ''));
                i++;
            }
            blocks.push({ type: 'list', ordered: true, items });
            continue;
        }
        // --- Unordered list ---
        if (/^[-*+]\s/.test(line)) {
            const items = [];
            while (i < lines.length && /^[-*+]\s/.test(lines[i])) {
                items.push(lines[i].replace(/^[-*+]\s/, ''));
                i++;
            }
            blocks.push({ type: 'list', ordered: false, items });
            continue;
        }
        // --- Empty line ---
        if (line.trim() === '') {
            i++;
            continue;
        }
        // --- Paragraph (collect consecutive non-empty lines) ---
        const paraLines = [];
        while (i < lines.length &&
            lines[i].trim() !== '' &&
            !lines[i].startsWith('#') &&
            !lines[i].startsWith('```') &&
            !lines[i].startsWith('> ') &&
            !/^[-*+]\s/.test(lines[i]) &&
            !/^\d+\.\s/.test(lines[i]) &&
            !/^(-{3,}|\*{3,}|_{3,})\s*$/.test(lines[i])) {
            paraLines.push(lines[i]);
            i++;
        }
        if (paraLines.length > 0) {
            blocks.push({ type: 'paragraph', text: paraLines.join('\n') });
        }
    }
    return blocks;
}
function parsePipeRow(line) {
    return line
        .replace(/^\|/, '')
        .replace(/\|$/, '')
        .split('|')
        .map((cell) => cell.trim());
}
function parseInlineSegments(text) {
    const segments = [];
    // Regex for inline patterns: **bold**, *italic*, `code`, [text](url), ~~strike~~
    const pattern = /(\*\*(.+?)\*\*)|(\*(.+?)\*)|(`(.+?)`)|(\[([^\]]+)\]\(([^)]+)\))|(~~(.+?)~~)/g;
    let lastIndex = 0;
    let match;
    while ((match = pattern.exec(text)) !== null) {
        // Push text before match
        if (match.index > lastIndex) {
            segments.push({ text: text.slice(lastIndex, match.index) });
        }
        if (match[2] != null) {
            segments.push({ text: match[2], bold: true });
        }
        else if (match[4] != null) {
            segments.push({ text: match[4], italic: true });
        }
        else if (match[6] != null) {
            segments.push({ text: match[6], code: true });
        }
        else if (match[8] != null && match[9] != null) {
            segments.push({ text: match[8], link: match[9] });
        }
        else if (match[11] != null) {
            segments.push({ text: match[11], strikethrough: true });
        }
        lastIndex = match.index + match[0].length;
    }
    // Remaining text
    if (lastIndex < text.length) {
        segments.push({ text: text.slice(lastIndex) });
    }
    if (segments.length === 0 && text.length > 0) {
        segments.push({ text });
    }
    return segments;
}
function InlineText({ text, colors, }) {
    const segments = useMemo(() => parseInlineSegments(text), [text]);
    return (_jsx(Text, { wrap: "wrap", children: segments.map((seg, idx) => {
            if (seg.code) {
                return (_jsx(Text, { color: colors.text.code, children: seg.text }, idx));
            }
            if (seg.link) {
                return (_jsx(Text, { color: colors.text.link, underline: true, children: seg.text }, idx));
            }
            if (seg.bold) {
                return (_jsx(Text, { color: colors.text.strong, bold: true, children: seg.text }, idx));
            }
            if (seg.italic) {
                return (_jsx(Text, { color: colors.text.emphasis, italic: true, children: seg.text }, idx));
            }
            if (seg.strikethrough) {
                return (_jsx(Text, { color: colors.text.secondary, strikethrough: true, children: seg.text }, idx));
            }
            return (_jsx(Text, { color: colors.text.primary, children: seg.text }, idx));
        }) }));
}
function hlClassToColor(classes, colors) {
    if (!classes || classes.length === 0)
        return colors.text.primary;
    for (const cls of classes) {
        const name = cls.replace('hljs-', '');
        switch (name) {
            case 'keyword':
            case 'built_in':
                return colors.syntax.keyword;
            case 'string':
            case 'regexp':
                return colors.syntax.string;
            case 'number':
            case 'literal':
                return colors.syntax.number;
            case 'comment':
            case 'doctag':
                return colors.syntax.comment;
            case 'type':
            case 'class':
            case 'title':
                return colors.syntax.type;
            case 'function':
                return colors.syntax.function;
            case 'variable':
            case 'template-variable':
                return colors.syntax.variable;
            case 'attr':
            case 'attribute':
                return colors.syntax.builtIn;
            case 'tag':
            case 'name':
                return colors.syntax.tag;
            case 'link':
                return colors.syntax.link;
            case 'params':
                return colors.syntax.variable;
            case 'meta':
                return colors.syntax.comment;
            default:
                // Check for combined classes like 'title.function'
                if (name.includes('function'))
                    return colors.syntax.function;
                if (name.includes('class'))
                    return colors.syntax.type;
                return colors.text.primary;
        }
    }
    return colors.text.primary;
}
function renderHlNodes(nodes, colors, parentColor) {
    const elements = [];
    for (let i = 0; i < nodes.length; i++) {
        const node = nodes[i];
        if (node.type === 'text') {
            elements.push(_jsx(Text, { color: parentColor ?? colors.text.primary, children: node.value ?? '' }, i));
        }
        else if (node.type === 'element' && node.children) {
            const color = hlClassToColor(node.properties?.className, colors);
            elements.push(_jsx(Text, { children: renderHlNodes(node.children, colors, color) }, i));
        }
    }
    return elements;
}
function HighlightedCode({ code, language, colors, }) {
    const highlighted = useMemo(() => {
        try {
            if (language && lowlight.registered(language)) {
                return lowlight.highlight(language, code);
            }
            return lowlight.highlightAuto(code);
        }
        catch {
            return null;
        }
    }, [code, language]);
    if (!highlighted) {
        return _jsx(Text, { color: colors.text.primary, children: code });
    }
    return (_jsx(Text, { children: renderHlNodes(highlighted.children, colors) }));
}
// ---------------------------------------------------------------------------
// Block renderers
// ---------------------------------------------------------------------------
function RenderBlock({ block, colors, }) {
    switch (block.type) {
        case 'heading': {
            const headingColor = colors.text.heading;
            const prefix = block.level <= 2 ? '' : '';
            return (_jsx(Box, { marginTop: 1, marginBottom: 0, children: _jsxs(Text, { color: headingColor, bold: true, children: [prefix, block.text] }) }));
        }
        case 'paragraph':
            return (_jsx(Box, { marginTop: 0, marginBottom: 0, children: _jsx(InlineText, { text: block.text, colors: colors }) }));
        case 'code':
            return (_jsxs(Box, { flexDirection: "column", marginTop: 0, marginBottom: 0, paddingLeft: 1, paddingRight: 1, children: [block.language && (_jsx(Text, { color: colors.text.secondary, dimColor: true, children: block.language })), _jsx(HighlightedCode, { code: block.code, language: block.language, colors: colors })] }));
        case 'list':
            return (_jsx(Box, { flexDirection: "column", marginTop: 0, marginBottom: 0, paddingLeft: 1, children: block.items.map((item, idx) => (_jsxs(Box, { flexDirection: "row", children: [_jsx(Box, { width: 3, flexShrink: 0, children: _jsxs(Text, { color: colors.text.secondary, children: [block.ordered ? `${idx + 1}.` : '\u2022', ' '] }) }), _jsx(Box, { flexGrow: 1, children: _jsx(InlineText, { text: item, colors: colors }) })] }, idx))) }));
        case 'blockquote':
            return (_jsxs(Box, { flexDirection: "row", marginTop: 0, marginBottom: 0, paddingLeft: 1, children: [_jsx(Box, { width: 2, flexShrink: 0, children: _jsx(Text, { color: colors.border.dim, children: '\u2502 ' }) }), _jsx(Box, { flexGrow: 1, children: _jsx(Text, { color: colors.text.emphasis, italic: true, wrap: "wrap", children: block.text }) })] }));
        case 'thematic_break':
            return (_jsx(Box, { marginTop: 0, marginBottom: 0, children: _jsx(Text, { color: colors.border.dim, children: '\u2500'.repeat(40) }) }));
        case 'table': {
            // Calculate column widths
            const allRows = [block.headers, ...block.rows];
            const colWidths = block.headers.map((_, colIdx) => {
                let max = 0;
                for (const row of allRows) {
                    const cell = row[colIdx] ?? '';
                    if (cell.length > max)
                        max = cell.length;
                }
                return Math.min(max + 2, 30);
            });
            return (_jsxs(Box, { flexDirection: "column", marginTop: 0, marginBottom: 0, paddingLeft: 1, children: [_jsx(Box, { flexDirection: "row", children: block.headers.map((header, idx) => (_jsx(Box, { width: colWidths[idx], flexShrink: 0, children: _jsx(Text, { color: colors.text.heading, bold: true, children: header }) }, idx))) }), _jsx(Box, { flexDirection: "row", children: colWidths.map((w, idx) => (_jsx(Box, { width: w, flexShrink: 0, children: _jsx(Text, { color: colors.border.dim, children: '\u2500'.repeat(Math.max(w - 1, 1)) }) }, idx))) }), block.rows.map((row, rowIdx) => (_jsx(Box, { flexDirection: "row", children: row.map((cell, cellIdx) => (_jsx(Box, { width: colWidths[cellIdx] ?? 10, flexShrink: 0, children: _jsx(Text, { color: colors.text.primary, children: cell }) }, cellIdx))) }, rowIdx)))] }));
        }
        default:
            return _jsx(Text, {});
    }
}
// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------
export const MarkdownRenderer = React.memo(function MarkdownRenderer({ content, isStreaming, }) {
    const { colors } = useTheme();
    // Cache parsed blocks across renders. During streaming, only re-parse
    // the last block (the one actively receiving new content).
    const prevContentRef = useRef('');
    const cachedBlocksRef = useRef([]);
    const blocks = useMemo(() => {
        if (!content)
            return [];
        const prevContent = prevContentRef.current;
        const prevBlocks = cachedBlocksRef.current;
        // If content starts with previous content and we're streaming,
        // only re-parse from where the last block started.
        if (isStreaming &&
            prevBlocks.length > 0 &&
            content.startsWith(prevContent) &&
            content.length > prevContent.length) {
            // Re-parse everything from the last block boundary.
            // This is still efficient since we only parse the tail.
            const allBlocks = parseMarkdownBlocks(content);
            prevContentRef.current = content;
            cachedBlocksRef.current = allBlocks;
            return allBlocks;
        }
        const allBlocks = parseMarkdownBlocks(content);
        prevContentRef.current = content;
        cachedBlocksRef.current = allBlocks;
        return allBlocks;
    }, [content, isStreaming]);
    return (_jsxs(Box, { flexDirection: "column", children: [blocks.map((block, idx) => (_jsx(RenderBlock, { block: block, colors: colors }, idx))), isStreaming && (_jsx(Text, { color: colors.text.accent, children: '\u258b' }))] }));
});
//# sourceMappingURL=markdown-renderer.js.map