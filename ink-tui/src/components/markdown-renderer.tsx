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
import type { SemanticColors } from '../themes/semantic-tokens.js';
import { common, createLowlight } from 'lowlight';

// ---------------------------------------------------------------------------
// lowlight singleton
// ---------------------------------------------------------------------------

const lowlight = createLowlight(common);

// ---------------------------------------------------------------------------
// Block types
// ---------------------------------------------------------------------------

interface HeadingBlock {
  type: 'heading';
  level: number;
  text: string;
}

interface ParagraphBlock {
  type: 'paragraph';
  text: string;
}

interface CodeBlock {
  type: 'code';
  language: string;
  code: string;
}

interface ListBlock {
  type: 'list';
  ordered: boolean;
  items: string[];
}

interface BlockquoteBlock {
  type: 'blockquote';
  text: string;
}

interface ThematicBreakBlock {
  type: 'thematic_break';
}

interface TableBlock {
  type: 'table';
  headers: string[];
  rows: string[][];
}

type MarkdownBlock =
  | HeadingBlock
  | ParagraphBlock
  | CodeBlock
  | ListBlock
  | BlockquoteBlock
  | ThematicBreakBlock
  | TableBlock;

// ---------------------------------------------------------------------------
// Parser — line-by-line block extraction
// ---------------------------------------------------------------------------

function parseMarkdownBlocks(source: string): MarkdownBlock[] {
  const lines = source.split('\n');
  const blocks: MarkdownBlock[] = [];
  let i = 0;

  while (i < lines.length) {
    const line = lines[i]!;

    // --- Fenced code block ---
    const fenceMatch = line.match(/^```(\w*)/);
    if (fenceMatch) {
      const language = fenceMatch[1] ?? '';
      const codeLines: string[] = [];
      i++;
      while (i < lines.length && !lines[i]!.startsWith('```')) {
        codeLines.push(lines[i]!);
        i++;
      }
      if (i < lines.length) i++; // skip closing fence
      blocks.push({ type: 'code', language, code: codeLines.join('\n') });
      continue;
    }

    // --- Heading ---
    const headingMatch = line.match(/^(#{1,6})\s+(.*)/);
    if (headingMatch) {
      blocks.push({
        type: 'heading',
        level: headingMatch[1]!.length,
        text: headingMatch[2]!,
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
    if (line.includes('|') && i + 1 < lines.length && /^\|?\s*[-:]+/.test(lines[i + 1]!)) {
      const headers = parsePipeRow(line);
      i += 2; // skip header + separator
      const rows: string[][] = [];
      while (i < lines.length && lines[i]!.includes('|')) {
        rows.push(parsePipeRow(lines[i]!));
        i++;
      }
      blocks.push({ type: 'table', headers, rows });
      continue;
    }

    // --- Blockquote ---
    if (line.startsWith('> ') || line === '>') {
      const quoteLines: string[] = [];
      while (i < lines.length && (lines[i]!.startsWith('> ') || lines[i] === '>')) {
        quoteLines.push(lines[i]!.replace(/^>\s?/, ''));
        i++;
      }
      blocks.push({ type: 'blockquote', text: quoteLines.join('\n') });
      continue;
    }

    // --- Ordered list ---
    if (/^\d+\.\s/.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^\d+\.\s/.test(lines[i]!)) {
        items.push(lines[i]!.replace(/^\d+\.\s/, ''));
        i++;
      }
      blocks.push({ type: 'list', ordered: true, items });
      continue;
    }

    // --- Unordered list ---
    if (/^[-*+]\s/.test(line)) {
      const items: string[] = [];
      while (i < lines.length && /^[-*+]\s/.test(lines[i]!)) {
        items.push(lines[i]!.replace(/^[-*+]\s/, ''));
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
    const paraLines: string[] = [];
    while (
      i < lines.length &&
      lines[i]!.trim() !== '' &&
      !lines[i]!.startsWith('#') &&
      !lines[i]!.startsWith('```') &&
      !lines[i]!.startsWith('> ') &&
      !/^[-*+]\s/.test(lines[i]!) &&
      !/^\d+\.\s/.test(lines[i]!) &&
      !/^(-{3,}|\*{3,}|_{3,})\s*$/.test(lines[i]!)
    ) {
      paraLines.push(lines[i]!);
      i++;
    }
    if (paraLines.length > 0) {
      blocks.push({ type: 'paragraph', text: paraLines.join('\n') });
    }
  }

  return blocks;
}

function parsePipeRow(line: string): string[] {
  return line
    .replace(/^\|/, '')
    .replace(/\|$/, '')
    .split('|')
    .map((cell) => cell.trim());
}

// ---------------------------------------------------------------------------
// Inline rendering — bold, italic, code, links
// ---------------------------------------------------------------------------

interface InlineSegment {
  text: string;
  bold?: boolean;
  italic?: boolean;
  code?: boolean;
  link?: string;
  strikethrough?: boolean;
}

function parseInlineSegments(text: string): InlineSegment[] {
  const segments: InlineSegment[] = [];
  // Regex for inline patterns: **bold**, *italic*, `code`, [text](url), ~~strike~~
  const pattern = /(\*\*(.+?)\*\*)|(\*(.+?)\*)|(`(.+?)`)|(\[([^\]]+)\]\(([^)]+)\))|(~~(.+?)~~)/g;
  let lastIndex = 0;
  let match: RegExpExecArray | null;

  while ((match = pattern.exec(text)) !== null) {
    // Push text before match
    if (match.index > lastIndex) {
      segments.push({ text: text.slice(lastIndex, match.index) });
    }

    if (match[2] != null) {
      segments.push({ text: match[2], bold: true });
    } else if (match[4] != null) {
      segments.push({ text: match[4], italic: true });
    } else if (match[6] != null) {
      segments.push({ text: match[6], code: true });
    } else if (match[8] != null && match[9] != null) {
      segments.push({ text: match[8], link: match[9] });
    } else if (match[11] != null) {
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

function InlineText({
  text,
  colors,
}: {
  text: string;
  colors: SemanticColors;
}): React.ReactElement {
  const segments = useMemo(() => parseInlineSegments(text), [text]);

  return (
    <Text wrap="wrap">
      {segments.map((seg, idx) => {
        if (seg.code) {
          return (
            <Text key={idx} color={colors.text.code}>
              {seg.text}
            </Text>
          );
        }
        if (seg.link) {
          return (
            <Text key={idx} color={colors.text.link} underline>
              {seg.text}
            </Text>
          );
        }
        if (seg.bold) {
          return (
            <Text key={idx} color={colors.text.strong} bold>
              {seg.text}
            </Text>
          );
        }
        if (seg.italic) {
          return (
            <Text key={idx} color={colors.text.emphasis} italic>
              {seg.text}
            </Text>
          );
        }
        if (seg.strikethrough) {
          return (
            <Text key={idx} color={colors.text.secondary} strikethrough>
              {seg.text}
            </Text>
          );
        }
        return (
          <Text key={idx} color={colors.text.primary}>
            {seg.text}
          </Text>
        );
      })}
    </Text>
  );
}

// ---------------------------------------------------------------------------
// Code block rendering with lowlight
// ---------------------------------------------------------------------------

interface HlNode {
  type: string;
  tagName?: string;
  properties?: { className?: string[] };
  children?: HlNode[];
  value?: string;
}

function hlClassToColor(classes: string[] | undefined, colors: SemanticColors): string {
  if (!classes || classes.length === 0) return colors.text.primary;

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
        if (name.includes('function')) return colors.syntax.function;
        if (name.includes('class')) return colors.syntax.type;
        return colors.text.primary;
    }
  }

  return colors.text.primary;
}

function renderHlNodes(
  nodes: HlNode[],
  colors: SemanticColors,
  parentColor?: string,
): React.ReactElement[] {
  const elements: React.ReactElement[] = [];

  for (let i = 0; i < nodes.length; i++) {
    const node = nodes[i]!;
    if (node.type === 'text') {
      elements.push(
        <Text key={i} color={parentColor ?? colors.text.primary}>
          {node.value ?? ''}
        </Text>,
      );
    } else if (node.type === 'element' && node.children) {
      const color = hlClassToColor(node.properties?.className, colors);
      elements.push(
        <Text key={i}>
          {renderHlNodes(node.children, colors, color)}
        </Text>,
      );
    }
  }

  return elements;
}

function HighlightedCode({
  code,
  language,
  colors,
}: {
  code: string;
  language: string;
  colors: SemanticColors;
}): React.ReactElement {
  const highlighted = useMemo(() => {
    try {
      if (language && lowlight.registered(language)) {
        return lowlight.highlight(language, code);
      }
      return lowlight.highlightAuto(code);
    } catch {
      return null;
    }
  }, [code, language]);

  if (!highlighted) {
    return <Text color={colors.text.primary}>{code}</Text>;
  }

  return (
    <Text>
      {renderHlNodes(highlighted.children as HlNode[], colors)}
    </Text>
  );
}

// ---------------------------------------------------------------------------
// Block renderers
// ---------------------------------------------------------------------------

function RenderBlock({
  block,
  colors,
}: {
  block: MarkdownBlock;
  colors: SemanticColors;
}): React.ReactElement {
  switch (block.type) {
    case 'heading': {
      const headingColor = colors.text.heading;
      const prefix = block.level <= 2 ? '' : '';
      return (
        <Box marginTop={1} marginBottom={0}>
          <Text color={headingColor} bold>
            {prefix}{block.text}
          </Text>
        </Box>
      );
    }

    case 'paragraph':
      return (
        <Box marginTop={0} marginBottom={0}>
          <InlineText text={block.text} colors={colors} />
        </Box>
      );

    case 'code':
      return (
        <Box
          flexDirection="column"
          marginTop={0}
          marginBottom={0}
          paddingLeft={1}
          paddingRight={1}
        >
          {block.language && (
            <Text color={colors.text.secondary} dimColor>
              {block.language}
            </Text>
          )}
          <HighlightedCode
            code={block.code}
            language={block.language}
            colors={colors}
          />
        </Box>
      );

    case 'list':
      return (
        <Box flexDirection="column" marginTop={0} marginBottom={0} paddingLeft={1}>
          {block.items.map((item, idx) => (
            <Box key={idx} flexDirection="row">
              <Box width={3} flexShrink={0}>
                <Text color={colors.text.secondary}>
                  {block.ordered ? `${idx + 1}.` : '\u2022'}
                  {' '}
                </Text>
              </Box>
              <Box flexGrow={1}>
                <InlineText text={item} colors={colors} />
              </Box>
            </Box>
          ))}
        </Box>
      );

    case 'blockquote':
      return (
        <Box
          flexDirection="row"
          marginTop={0}
          marginBottom={0}
          paddingLeft={1}
        >
          <Box width={2} flexShrink={0}>
            <Text color={colors.border.dim}>{'\u2502 '}</Text>
          </Box>
          <Box flexGrow={1}>
            <Text color={colors.text.emphasis} italic wrap="wrap">
              {block.text}
            </Text>
          </Box>
        </Box>
      );

    case 'thematic_break':
      return (
        <Box marginTop={0} marginBottom={0}>
          <Text color={colors.border.dim}>
            {'\u2500'.repeat(40)}
          </Text>
        </Box>
      );

    case 'table': {
      // Calculate column widths
      const allRows = [block.headers, ...block.rows];
      const colWidths = block.headers.map((_, colIdx) => {
        let max = 0;
        for (const row of allRows) {
          const cell = row[colIdx] ?? '';
          if (cell.length > max) max = cell.length;
        }
        return Math.min(max + 2, 30);
      });

      return (
        <Box flexDirection="column" marginTop={0} marginBottom={0} paddingLeft={1}>
          {/* Header */}
          <Box flexDirection="row">
            {block.headers.map((header, idx) => (
              <Box key={idx} width={colWidths[idx]} flexShrink={0}>
                <Text color={colors.text.heading} bold>
                  {header}
                </Text>
              </Box>
            ))}
          </Box>
          {/* Separator */}
          <Box flexDirection="row">
            {colWidths.map((w, idx) => (
              <Box key={idx} width={w} flexShrink={0}>
                <Text color={colors.border.dim}>
                  {'\u2500'.repeat(Math.max(w - 1, 1))}
                </Text>
              </Box>
            ))}
          </Box>
          {/* Rows */}
          {block.rows.map((row, rowIdx) => (
            <Box key={rowIdx} flexDirection="row">
              {row.map((cell, cellIdx) => (
                <Box key={cellIdx} width={colWidths[cellIdx] ?? 10} flexShrink={0}>
                  <Text color={colors.text.primary}>{cell}</Text>
                </Box>
              ))}
            </Box>
          ))}
        </Box>
      );
    }

    default:
      return <Text />;
  }
}

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface MarkdownRendererProps {
  /** The raw markdown string to render. */
  content: string;
  /** Whether the content is still being streamed. */
  isStreaming: boolean;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export const MarkdownRenderer = React.memo(function MarkdownRenderer({
  content,
  isStreaming,
}: MarkdownRendererProps): React.ReactElement {
  const { colors } = useTheme();

  // Cache parsed blocks across renders. During streaming, only re-parse
  // the last block (the one actively receiving new content).
  const prevContentRef = useRef('');
  const cachedBlocksRef = useRef<MarkdownBlock[]>([]);

  const blocks = useMemo(() => {
    if (!content) return [];

    const prevContent = prevContentRef.current;
    const prevBlocks = cachedBlocksRef.current;

    // If content starts with previous content and we're streaming,
    // only re-parse from where the last block started.
    if (
      isStreaming &&
      prevBlocks.length > 0 &&
      content.startsWith(prevContent) &&
      content.length > prevContent.length
    ) {
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

  return (
    <Box flexDirection="column">
      {blocks.map((block, idx) => (
        <RenderBlock key={idx} block={block} colors={colors} />
      ))}
      {isStreaming && (
        <Text color={colors.text.accent}>{'\u258b'}</Text>
      )}
    </Box>
  );
});
