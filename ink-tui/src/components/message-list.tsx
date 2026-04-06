/**
 * MessageList — renders the chat message array, dispatching to the
 * correct component for each message type.
 *
 * Message types:
 *  - user       -> UserMessage
 *  - assistant  -> AssistantMessage
 *  - system     -> SystemMessage
 *  - tool_call  -> ToolCard
 *  - kb_result  -> KnowledgeCard
 *  - banner     -> Banner
 */

import React from 'react';
import { Box } from 'ink';
import type { ChatMessage } from '../types/chat.js';
import { UserMessage } from './user-message.js';
import { AssistantMessage } from './assistant-message.js';
import { SystemMessage } from './system-message.js';
import { Banner } from './banner.js';
import { ToolCard } from './tool-card.js';
import { KnowledgeCard } from './knowledge-card.js';
import { FileOutput } from './file-output.js';
import { useTheme } from '../contexts/theme-context.js';
import { useTerminal } from '../contexts/terminal-context.js';
import { useChatActions } from '../contexts/chat-context.js';
import { useCredits } from '../hooks/use-credits.js';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface MessageListProps {
  messages: readonly ChatMessage[];
  focusedIndex: number;
}

// ---------------------------------------------------------------------------
// Individual message dispatch
// ---------------------------------------------------------------------------

const MessageItem = React.memo(function MessageItem({
  message,
  isFocused,
}: {
  message: ChatMessage;
  isFocused: boolean;
}): React.ReactElement | null {
  const terminal = useTerminal();
  const chatActions = useChatActions();
  const credits = useCredits();

  switch (message.kind) {
    case 'user':
      return (
        <UserMessage
          text={message.text}
          isSlashCommand={message.isSlashCommand}
          isFocused={isFocused}
        />
      );

    case 'assistant':
      return (
        <AssistantMessage
          content={message.content}
          streaming={message.streaming}
          isFocused={isFocused}
        />
      );

    case 'system':
      return (
        <SystemMessage
          text={message.text}
          level={message.level}
          isFocused={isFocused}
        />
      );

    case 'tool_call':
      return (
        <ToolCard
          toolId={message.toolId}
          toolName={message.toolName}
          status={message.status}
          input={message.inputPreview}
          output={message.output || undefined}
          durationMs={message.durationMs}
          diff={message.diff}
          expanded={message.expanded}
          onToggleExpand={() => {
            chatActions.toggleToolCardExpand(message.toolId);
          }}
          isFocused={isFocused}
        />
      );

    case 'kb_result':
      return (
        <KnowledgeCard
          queryId={String(message.queryId)}
          subQuestions={message.subQuestions.length > 0
            ? message.subQuestions.map((sq) => ({
                question: sq.subQuestion,
                results: [...sq.results],
              }))
            : [{ question: message.query, results: [] }]
          }
          answer={message.answer ?? ''}
          cached={message.fromCache}
          graphExpanded={message.subQuestions.some((sq) => sq.results.some((r) => r.graphExpanded))}
          durationMs={message.latencyMs}
          expanded={message.expanded}
          activeTabIndex={message.activeTab}
          onToggleExpand={() => {
            chatActions.toggleKBExpand(message.id);
          }}
          onTabChange={(index: number) => {
            chatActions.setKBActiveTab(message.id, index);
          }}
          isFocused={isFocused}
        />
      );

    case 'file_output':
      return (
        <FileOutput
          fileType={message.fileType}
          description={message.description}
          filePath={message.filePath}
          isFocused={isFocused}
        />
      );

    case 'banner':
      return (
        <Banner
          version={message.version}
          username={message.displayName}
          email={message.email}
          org={message.organization}
          workingDir={message.workingDir}
          provider={message.provider}
          modelDisplay={message.modelDisplay}
          credits={credits.balance !== 'checking...' && credits.balance !== 'API credits'
            ? (credits.provider !== 'unknown' ? `${credits.provider}: ${credits.balance}` : credits.balance)
            : (message.credits || credits.balance)}
          tips={[...message.tips]}
          terminalWidth={terminal.width}
        />
      );

    default:
      return null;
  }
});

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export const MessageList = React.memo(function MessageList({
  messages,
  focusedIndex,
}: MessageListProps): React.ReactElement {
  const { colors } = useTheme();

  return (
    <Box flexDirection="column">
      {messages.map((msg, idx) => {
        const isFocused = idx === focusedIndex;
        return (
          <Box
            key={msg.id}
            flexDirection="column"
            {...(isFocused
              ? { borderStyle: 'single' as const, borderColor: colors.border.focus, borderLeft: true, borderRight: false, borderTop: false, borderBottom: false }
              : {})}
          >
            <MessageItem message={msg} isFocused={isFocused} />
          </Box>
        );
      })}
    </Box>
  );
});
