/**
 * Base JSON-RPC protocol for Rust engine <-> Ink TUI communication.
 *
 * All messages are newline-delimited JSON (JSON Lines / NDJSON).
 * Direction: Engine -> TUI (events) and TUI -> Engine (actions).
 *
 * The Rust engine runs as a child process. The Ink TUI writes actions
 * to the engine's stdin and reads events from its stdout, one JSON
 * object per line.
 */

import { z } from 'zod';

// ---------------------------------------------------------------------------
// Base message envelope
// ---------------------------------------------------------------------------

/**
 * Every message — in both directions — carries a `type` discriminator,
 * an optional correlation `id`, and a millisecond-precision timestamp.
 */
export const BaseMessageSchema = z.object({
  /** Discriminator string — matches the event/action name. */
  type: z.string(),
  /** Optional correlation ID for request/response pairing. */
  id: z.string().optional(),
  /** Unix epoch milliseconds when the message was created. Rust does NOT send this — defaults to Date.now(). */
  timestamp: z.number().optional().default(() => Date.now()),
});

export type BaseMessage = z.infer<typeof BaseMessageSchema>;

// ---------------------------------------------------------------------------
// Direction markers (type-level only, not runtime)
// ---------------------------------------------------------------------------

/** Marker: message flows from Rust engine to Ink TUI. */
export type EngineEvent<T extends string = string> = BaseMessage & { type: T };

/** Marker: message flows from Ink TUI to Rust engine. */
export type TuiAction<T extends string = string> = BaseMessage & { type: T };

// ---------------------------------------------------------------------------
// Protocol constants
// ---------------------------------------------------------------------------

/** All event type discriminators (Engine -> TUI). */
export const ENGINE_EVENT_TYPES = [
  'stream_delta',
  'stream_end',
  'tool_call_start',
  'tool_call_update',
  'tool_call_end',
  'permission_request',
  'ask_user_request',
  'status_update',
  'agent_spawned',
  'agent_status_changed',
  'agent_completed',
  'agent_failed',
  'usage_update',
  'knowledge_result',
  'system_message',
  'banner',
  'sidebar_update',
  'model_info',
  'context_files_update',
] as const;

/** All action type discriminators (TUI -> Engine). */
export const TUI_ACTION_TYPES = [
  'submit_prompt',
  'run_in_background',
  'cancel_agent',
  'permission_response',
  'ask_user_response',
  'knowledge_feedback',
  'update_permissions',
  'toggle_context_file',
  'change_routing',
  'clear_chat',
  'slash_command',
  'update_model',
  'moe_dispatch',
  'inject_skill',
  'voice_transcribed',
  'quit',
] as const;

export type EngineEventType = (typeof ENGINE_EVENT_TYPES)[number];
export type TuiActionType = (typeof TUI_ACTION_TYPES)[number];

// ---------------------------------------------------------------------------
// Connection state
// ---------------------------------------------------------------------------

export const ConnectionState = z.enum([
  'connecting',
  'connected',
  'disconnected',
  'error',
]);

export type ConnectionState = z.infer<typeof ConnectionState>;

// ---------------------------------------------------------------------------
// Utility: wrap a payload schema into a full message schema
// ---------------------------------------------------------------------------

/**
 * Creates a full message schema by merging the base envelope with a
 * typed payload. The `type` field is narrowed to the literal string.
 */
export function messageSchema<T extends string, P extends z.ZodRawShape>(
  typeLiteral: T,
  payloadShape: P,
) {
  return BaseMessageSchema.extend({
    type: z.literal(typeLiteral),
    ...payloadShape,
  });
}
