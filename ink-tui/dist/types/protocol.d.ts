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
/**
 * Every message — in both directions — carries a `type` discriminator,
 * an optional correlation `id`, and a millisecond-precision timestamp.
 */
export declare const BaseMessageSchema: z.ZodObject<{
    /** Discriminator string — matches the event/action name. */
    type: z.ZodString;
    /** Optional correlation ID for request/response pairing. */
    id: z.ZodOptional<z.ZodString>;
    /** Unix epoch milliseconds when the message was created. */
    timestamp: z.ZodNumber;
}, "strip", z.ZodTypeAny, {
    type: string;
    timestamp: number;
    id?: string | undefined;
}, {
    type: string;
    timestamp: number;
    id?: string | undefined;
}>;
export type BaseMessage = z.infer<typeof BaseMessageSchema>;
/** Marker: message flows from Rust engine to Ink TUI. */
export type EngineEvent<T extends string = string> = BaseMessage & {
    type: T;
};
/** Marker: message flows from Ink TUI to Rust engine. */
export type TuiAction<T extends string = string> = BaseMessage & {
    type: T;
};
/** All event type discriminators (Engine -> TUI). */
export declare const ENGINE_EVENT_TYPES: readonly ["stream_delta", "stream_end", "tool_call_start", "tool_call_update", "tool_call_complete", "permission_request", "ask_user_request", "status_update", "agent_spawned", "agent_status_changed", "agent_completed", "agent_failed", "usage_update", "kb_result", "system_message", "banner", "sidebar_update", "model_info", "context_files_update"];
/** All action type discriminators (TUI -> Engine). */
export declare const TUI_ACTION_TYPES: readonly ["submit_prompt", "run_in_background", "cancel_agent", "resolve_permission", "resolve_ask_user", "kb_feedback", "change_permission_mode", "toggle_context_file", "change_routing", "clear_chat", "slash_command", "update_model", "moe_dispatch", "inject_skill", "quit"];
export type EngineEventType = (typeof ENGINE_EVENT_TYPES)[number];
export type TuiActionType = (typeof TUI_ACTION_TYPES)[number];
export declare const ConnectionState: z.ZodEnum<["connecting", "connected", "disconnected", "error"]>;
export type ConnectionState = z.infer<typeof ConnectionState>;
/**
 * Creates a full message schema by merging the base envelope with a
 * typed payload. The `type` field is narrowed to the literal string.
 */
export declare function messageSchema<T extends string, P extends z.ZodRawShape>(typeLiteral: T, payloadShape: P): z.ZodObject<z.objectUtil.extendShape<{
    /** Discriminator string — matches the event/action name. */
    type: z.ZodString;
    /** Optional correlation ID for request/response pairing. */
    id: z.ZodOptional<z.ZodString>;
    /** Unix epoch milliseconds when the message was created. */
    timestamp: z.ZodNumber;
}, {
    type: z.ZodLiteral<T>;
} & P>, "strip", z.ZodTypeAny, z.objectUtil.addQuestionMarks<z.baseObjectOutputType<z.objectUtil.extendShape<{
    /** Discriminator string — matches the event/action name. */
    type: z.ZodString;
    /** Optional correlation ID for request/response pairing. */
    id: z.ZodOptional<z.ZodString>;
    /** Unix epoch milliseconds when the message was created. */
    timestamp: z.ZodNumber;
}, {
    type: z.ZodLiteral<T>;
} & P>>, any> extends infer T_1 ? { [k in keyof T_1]: T_1[k]; } : never, z.baseObjectInputType<z.objectUtil.extendShape<{
    /** Discriminator string — matches the event/action name. */
    type: z.ZodString;
    /** Optional correlation ID for request/response pairing. */
    id: z.ZodOptional<z.ZodString>;
    /** Unix epoch milliseconds when the message was created. */
    timestamp: z.ZodNumber;
}, {
    type: z.ZodLiteral<T>;
} & P>> extends infer T_2 ? { [k_1 in keyof T_2]: T_2[k_1]; } : never>;
