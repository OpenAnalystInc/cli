/**
 * SidebarActivity -- Activity summary section for the sidebar panel.
 *
 * Matches Ratatui design exactly:
 *   updown-arrow N tool calls   (cyan icon)
 *   down-arrow N tokens         (green icon)
 *   clock Ns elapsed            (yellow icon)
 *   F mode: full-access         (red F for full-access, etc.)
 *
 * This section has no navigable items -- it is display-only.
 */

import React from 'react';
import { Box, Text } from 'ink';
import type { ActivityInfo } from '../types/messages.js';
import type { SemanticColors } from '../themes/semantic-tokens.js';
import { providerPreferences } from '../utils/provider-preferences.js';
import { PROVIDER_CONFIG } from '../utils/credential-manager.js';

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

export interface SidebarActivityProps {
  activity: ActivityInfo;
  isFocused: boolean;
  colors: SemanticColors;
  /** Current permission mode for display. */
  permissionMode?: string;
  /** Elapsed seconds since session start. */
  elapsedSecs?: number;
  /** Total tokens used. */
  totalTokens?: number;
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function formatTokens(tokens: number): string {
  if (tokens < 1_000) return String(tokens);
  if (tokens < 1_000_000) return `${(tokens / 1_000).toFixed(1)}k`;
  return `${(tokens / 1_000_000).toFixed(1)}M`;
}

function formatElapsed(secs: number): string {
  if (secs < 60) return `${secs}s`;
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return `${m}m ${String(s).padStart(2, '0')}s`;
}

function getPermissionDisplay(mode?: string): { icon: string; color: string; label: string } {
  switch (mode) {
    case 'read-only':
      return { icon: 'R', color: '#0088FF', label: 'read-only' };
    case 'workspace-write':
      return { icon: 'W', color: '#FFD700', label: 'workspace' };
    case 'prompt':
    case undefined:
      return { icon: 'P', color: '#00BFFF', label: 'prompt' };
    case 'danger-full-access':
      return { icon: 'F', color: '#FF4444', label: 'full-access' };
    default:
      return { icon: '?', color: '#888888', label: mode || 'unknown' };
  }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

export function SidebarActivity({
  activity,
  isFocused,
  colors,
  permissionMode,
  elapsedSecs = 0,
  totalTokens = 0,
}: SidebarActivityProps): React.ReactElement {
  const textColor = isFocused ? colors.sidebar.itemSelected : colors.text.primary;
  const perm = getPermissionDisplay(permissionMode);

  return (
    <Box flexDirection="column">
      {/* Tool calls */}
      <Box>
        <Text color="#0088FF"> {'\u21C5'} </Text>
        <Text color={textColor}>
          {activity.toolCallCount} tool calls
        </Text>
      </Box>
      {/* Tokens */}
      <Box>
        <Text color="#00CC44"> {'\u2193'} </Text>
        <Text color={textColor}>
          {formatTokens(totalTokens)} tokens
        </Text>
      </Box>
      {/* Elapsed */}
      <Box>
        <Text color="#FFD700"> {'\u2299'} </Text>
        <Text color={textColor}>
          {formatElapsed(elapsedSecs)} elapsed
        </Text>
      </Box>
      {/* Permission mode */}
      <Box>
        <Text color={perm.color}> {perm.icon} </Text>
        <Text color={textColor}>
          mode: {perm.label}
        </Text>
      </Box>
      {/* Default provider */}
      {(() => {
        const dp = providerPreferences.getDefaultProvider();
        const dpConfig = dp ? PROVIDER_CONFIG[dp] : null;
        if (dpConfig) {
          return (
            <Box>
              <Text color="#FFD700">{' \u2605'} </Text>
              <Text color={textColor}>
                {dpConfig.displayName}
              </Text>
            </Box>
          );
        }
        return null;
      })()}
      {/* Credit balance if present */}
      {activity.creditBalance != null && (
        <Box>
          <Text color={colors.text.secondary}> $</Text>
          <Text color={textColor}>
            {' '}{activity.creditBalance}
          </Text>
        </Box>
      )}
    </Box>
  );
}
