/**
 * Complete message type definitions for the Rust engine <-> Ink TUI protocol.
 *
 * Every event (Engine -> TUI) and action (TUI -> Engine) has:
 * 1. A Zod schema for runtime validation
 * 2. A TypeScript type inferred from the schema
 *
 * Schemas are grouped into discriminated unions so a single `parse()` call
 * can validate any incoming message and narrow its type.
 */

import { z } from 'zod';
import { BaseMessageSchema, messageSchema } from './protocol.js';

// ═══════════════════════════════════════════════════════════════════════════
// Shared / reusable schemas
// ═══════════════════════════════════════════════════════════════════════════

// -- Agent types (mirrors Rust `events::AgentType`) --

export const AgentTypeSchema = z.enum(['Primary', 'Explore', 'Plan', 'General']);
export type AgentType = z.infer<typeof AgentTypeSchema>;

// -- Agent status (mirrors Rust `events::AgentStatus`) --

export const AgentStatusSchema = z.enum(['Pending', 'Running', 'Completed', 'Failed']);
export type AgentStatus = z.infer<typeof AgentStatusSchema>;

// -- Diff structures (mirrors Rust `events::DiffInfo`) --

export const DiffLineSchema = z.discriminatedUnion('kind', [
  z.object({ kind: z.literal('context'), text: z.string() }),
  z.object({ kind: z.literal('added'), text: z.string() }),
  z.object({ kind: z.literal('removed'), text: z.string() }),
]);
export type DiffLine = z.infer<typeof DiffLineSchema>;

export const DiffHunkSchema = z.object({
  oldStart: z.number(),
  newStart: z.number(),
  lines: z.array(DiffLineSchema),
});
export type DiffHunk = z.infer<typeof DiffHunkSchema>;

export const DiffInfoSchema = z.object({
  filePath: z.string(),
  added: z.number(),
  removed: z.number(),
  hunks: z.array(DiffHunkSchema),
});
export type DiffInfo = z.infer<typeof DiffInfoSchema>;

// -- Knowledge base result entry --

export const KbChunkResultSchema = z.object({
  chunkId: z.string(),
  text: z.string(),
  snippet: z.string(),
  score: z.number().min(0).max(1),
  categoryLabel: z.string(),
  contentType: z.string(),
  citationLabel: z.string(),
  hasTimestamps: z.boolean(),
  graphExpanded: z.boolean(),
});
export type KbChunkResult = z.infer<typeof KbChunkResultSchema>;

export const SubQuestionResultSchema = z.object({
  subQuestion: z.string(),
  intent: z.string(),
  results: z.array(KbChunkResultSchema),
});
export type SubQuestionResult = z.infer<typeof SubQuestionResultSchema>;

// -- Agent info (sidebar) --

export const AgentInfoSchema = z.object({
  agentId: z.string(),
  agentType: AgentTypeSchema,
  taskSummary: z.string(),
  status: AgentStatusSchema,
});
export type AgentInfo = z.infer<typeof AgentInfoSchema>;

// -- File info (sidebar) --

export const FileActionSchema = z.enum(['read', 'edited', 'created']);
export type FileAction = z.infer<typeof FileActionSchema>;

export const FileInfoSchema = z.object({
  path: z.string(),
  action: FileActionSchema,
});
export type FileInfo = z.infer<typeof FileInfoSchema>;

// -- Plan info (sidebar) --

export const PlanInfoSchema = z.object({
  name: z.string(),
  status: z.enum(['todo', 'in_progress', 'done']),
});
export type PlanInfo = z.infer<typeof PlanInfoSchema>;

// -- Routing table (sidebar) --

export const ActionCategorySchema = z.enum(['explore', 'research', 'code', 'write']);
export type ActionCategory = z.infer<typeof ActionCategorySchema>;

export const RoutingEntrySchema = z.object({
  model: z.string(),
  tier: z.string(),
});
export type RoutingEntry = z.infer<typeof RoutingEntrySchema>;

export const RoutingTableSchema = z.object({
  explore: RoutingEntrySchema,
  research: RoutingEntrySchema,
  code: RoutingEntrySchema,
  write: RoutingEntrySchema,
});
export type RoutingTable = z.infer<typeof RoutingTableSchema>;

// -- Activity info (sidebar) --

export const ActivityInfoSchema = z.object({
  backgroundTasks: z.number(),
  toolCallCount: z.number(),
  mcpServers: z.number(),
  creditBalance: z.string().optional(),
});
export type ActivityInfo = z.infer<typeof ActivityInfoSchema>;

// -- Status phase (mirrors Rust `status_bar::AgentPhase`) --

export const AgentPhaseSchema = z.enum([
  'idle',
  'thinking',
  'reading_file',
  'editing_file',
  'running_bash',
  'searching',
  'done',
  'error',
]);
export type AgentPhase = z.infer<typeof AgentPhaseSchema>;

// -- Permission mode --

export const PermissionModeSchema = z.enum([
  'prompt',
  'read-only',
  'workspace-write',
  'danger-full-access',
]);
export type PermissionMode = z.infer<typeof PermissionModeSchema>;

// -- System message level --

export const SystemLevelSchema = z.enum(['info', 'warning', 'error']);
export type SystemLevel = z.infer<typeof SystemLevelSchema>;

// ═══════════════════════════════════════════════════════════════════════════
// Engine -> TUI events
// ═══════════════════════════════════════════════════════════════════════════

// 1. stream_delta — LLM response chunk
// NOTE: Rust sends { "text": "..." } not { "content": "..." }.
// Rust does NOT send a "done" field — the stream_end event signals completion.
export const StreamDeltaSchema = messageSchema('stream_delta', {
  agentId: z.string(),
  text: z.string(),
});
export type StreamDelta = z.infer<typeof StreamDeltaSchema>;

// 2. stream_end — assistant finished streaming
export const StreamEndSchema = messageSchema('stream_end', {
  agentId: z.string(),
});
export type StreamEnd = z.infer<typeof StreamEndSchema>;

// 3. tool_call_start — tool execution begins
// NOTE: Rust sends "callId" not "toolId".
export const ToolCallStartSchema = messageSchema('tool_call_start', {
  agentId: z.string(),
  callId: z.string(),
  toolName: z.string(),
  inputPreview: z.string(),
});
export type ToolCallStart = z.infer<typeof ToolCallStartSchema>;

// 4. tool_call_update — tool progress (partial output)
// NOTE: This event does NOT exist in Rust yet. Kept for future use.
export const ToolCallUpdateSchema = messageSchema('tool_call_update', {
  agentId: z.string(),
  callId: z.string(),
  output: z.string(),
});
export type ToolCallUpdate = z.infer<typeof ToolCallUpdateSchema>;

// 5. tool_call_end — tool finished
// NOTE: Rust sends "tool_call_end" not "tool_call_complete".
// Rust sends "callId" (not "toolId"), "isError" (bool, not "status" enum),
// and "duration" (ms number, not "durationMs").
export const ToolCallEndSchema = messageSchema('tool_call_end', {
  agentId: z.string(),
  callId: z.string(),
  output: z.string(),
  isError: z.boolean(),
  duration: z.number(),
  diff: DiffInfoSchema.optional().nullable(),
});
export type ToolCallEnd = z.infer<typeof ToolCallEndSchema>;

// Keep the old name as an alias for backward compatibility with mock mode
export const ToolCallCompleteSchema = ToolCallEndSchema;
export type ToolCallComplete = ToolCallEnd;

// 6. permission_request — needs user approval
// NOTE: Rust sends "input" not "toolInput", and "requiredMode" as a free string.
// Rust does NOT send "filePath" or "description".
export const PermissionRequestSchema = messageSchema('permission_request', {
  requestId: z.string(),
  agentId: z.string(),
  toolName: z.string(),
  input: z.string(),
  requiredMode: z.string(),
});
export type PermissionRequest = z.infer<typeof PermissionRequestSchema>;

// 7. ask_user_request — needs user choice or text input
// NOTE: Rust sends "default" not "defaultValue", and does NOT send "allowFreeText".
// Rust "options" is Option<Vec<String>> which serializes as null or array.
export const AskUserRequestSchema = messageSchema('ask_user_request', {
  requestId: z.string(),
  agentId: z.string(),
  question: z.string(),
  options: z.array(z.string()).optional().nullable(),
  default: z.string().optional().nullable(),
});
export type AskUserRequest = z.infer<typeof AskUserRequestSchema>;

// 8. status_update — phase change
export const StatusUpdateSchema = messageSchema('status_update', {
  phase: AgentPhaseSchema,
  label: z.string().optional(),
  elapsedMs: z.number(),
  tokensRemaining: z.number().optional(),
});
export type StatusUpdate = z.infer<typeof StatusUpdateSchema>;

// 9. agent_spawned — new agent created
export const AgentSpawnedSchema = messageSchema('agent_spawned', {
  agentId: z.string(),
  parentId: z.string().optional(),
  agentType: AgentTypeSchema,
  task: z.string(),
});
export type AgentSpawned = z.infer<typeof AgentSpawnedSchema>;

// 10. agent_status_changed — agent lifecycle update
export const AgentStatusChangedSchema = messageSchema('agent_status_changed', {
  agentId: z.string(),
  status: AgentStatusSchema,
});
export type AgentStatusChanged = z.infer<typeof AgentStatusChangedSchema>;

// 11. agent_completed — agent finished successfully
export const AgentCompletedSchema = messageSchema('agent_completed', {
  agentId: z.string(),
  result: z.string(),
});
export type AgentCompleted = z.infer<typeof AgentCompletedSchema>;

// 12. agent_failed — agent errored out
export const AgentFailedSchema = messageSchema('agent_failed', {
  agentId: z.string(),
  error: z.string(),
});
export type AgentFailed = z.infer<typeof AgentFailedSchema>;

// 13. usage_update — token usage from a streaming response
export const UsageUpdateSchema = messageSchema('usage_update', {
  agentId: z.string(),
  inputTokens: z.number(),
  outputTokens: z.number(),
});
export type UsageUpdate = z.infer<typeof UsageUpdateSchema>;

// 14. knowledge_result — knowledge base query result
// NOTE: Rust sends "knowledge_result" (from enum variant KnowledgeResult), not "kb_result".
export const KbResultSchema = messageSchema('knowledge_result', {
  queryId: z.number(),
  query: z.string(),
  intent: z.string(),
  subQuestions: z.array(SubQuestionResultSchema),
  answer: z.string().optional(),
  latencyMs: z.number(),
  fromCache: z.boolean(),
});
export type KbResult = z.infer<typeof KbResultSchema>;

// 15. system_message — system or error notice
export const SystemMessageSchema = messageSchema('system_message', {
  content: z.string(),
  level: SystemLevelSchema,
});
export type SystemMessage = z.infer<typeof SystemMessageSchema>;

// 16. banner — startup banner data
export const BannerSchema = messageSchema('banner', {
  version: z.string(),
  displayName: z.string(),
  email: z.string().optional(),
  organization: z.string().optional(),
  provider: z.string(),
  modelDisplay: z.string(),
  workingDir: z.string(),
  credits: z.string().optional(),
  tips: z.array(z.string()),
});
export type Banner = z.infer<typeof BannerSchema>;

// 17. sidebar_update — sidebar data refresh
export const SidebarUpdateSchema = messageSchema('sidebar_update', {
  agents: z.array(AgentInfoSchema),
  files: z.array(FileInfoSchema),
  plans: z.array(PlanInfoSchema),
  routing: RoutingTableSchema,
  activity: ActivityInfoSchema,
});
export type SidebarUpdate = z.infer<typeof SidebarUpdateSchema>;

// 18. model_info — current model details
export const ModelInfoSchema = messageSchema('model_info', {
  name: z.string(),
  provider: z.string(),
});
export type ModelInfo = z.infer<typeof ModelInfoSchema>;

// 19. context_files_update — context file list changed
export const ContextFilesUpdateSchema = messageSchema('context_files_update', {
  files: z.array(
    z.object({
      path: z.string(),
      action: z.enum(['added', 'removed']),
    }),
  ),
});
export type ContextFilesUpdate = z.infer<typeof ContextFilesUpdateSchema>;

// ═══════════════════════════════════════════════════════════════════════════
// Discriminated union of ALL engine events
// ═══════════════════════════════════════════════════════════════════════════

export const EngineEventSchema = z.discriminatedUnion('type', [
  StreamDeltaSchema,
  StreamEndSchema,
  ToolCallStartSchema,
  ToolCallUpdateSchema,
  ToolCallEndSchema,
  PermissionRequestSchema,
  AskUserRequestSchema,
  StatusUpdateSchema,
  AgentSpawnedSchema,
  AgentStatusChangedSchema,
  AgentCompletedSchema,
  AgentFailedSchema,
  UsageUpdateSchema,
  KbResultSchema,
  SystemMessageSchema,
  BannerSchema,
  SidebarUpdateSchema,
  ModelInfoSchema,
  ContextFilesUpdateSchema,
]);
export type EngineEvent = z.infer<typeof EngineEventSchema>;

// ═══════════════════════════════════════════════════════════════════════════
// TUI -> Engine actions
// ═══════════════════════════════════════════════════════════════════════════

// 1. submit_prompt — user sends a message
export const SubmitPromptSchema = messageSchema('submit_prompt', {
  text: z.string(),
  effortBudget: z.number().optional(),
  modelOverride: z.string().optional(),
});
export type SubmitPrompt = z.infer<typeof SubmitPromptSchema>;

// 2. run_in_background — submit async
export const RunInBackgroundSchema = messageSchema('run_in_background', {
  text: z.string(),
});
export type RunInBackground = z.infer<typeof RunInBackgroundSchema>;

// 3. cancel_agent — stop running agent
export const CancelAgentSchema = messageSchema('cancel_agent', {
  agentId: z.string().optional(),
});
export type CancelAgent = z.infer<typeof CancelAgentSchema>;

// 4. permission_response — answer permission dialog
// NOTE: Rust expects "permission_response" (not "resolve_permission"),
// with "requestId" and "allow" (boolean), not "decision" string.
export const ResolvePermissionSchema = messageSchema('permission_response', {
  requestId: z.string(),
  allow: z.boolean(),
});
export type ResolvePermission = z.infer<typeof ResolvePermissionSchema>;

// 5. ask_user_response — answer ask-user dialog
// NOTE: Rust expects "ask_user_response" (not "resolve_ask_user"),
// with "requestId" and "response" (not "answer").
export const ResolveAskUserSchema = messageSchema('ask_user_response', {
  requestId: z.string(),
  response: z.string(),
});
export type ResolveAskUser = z.infer<typeof ResolveAskUserSchema>;

// 6. knowledge_feedback — rate KB result
// NOTE: Rust expects "knowledge_feedback" (not "kb_feedback"),
// and "comment"/"correction" are required strings in Rust.
export const KbFeedbackSchema = messageSchema('knowledge_feedback', {
  queryId: z.number(),
  rating: z.enum(['positive', 'negative', 'corrected']),
  comment: z.string(),
  correction: z.string(),
});
export type KbFeedback = z.infer<typeof KbFeedbackSchema>;

// 7. update_permissions — Ctrl+P mode cycle
// NOTE: Rust expects "update_permissions" (not "change_permission_mode"),
// and it's a newtype variant wrapping a String. Internally tagged serde
// cannot deserialize newtype(String) -- this is a KNOWN Rust-side limitation.
// For now we send the object form and note this needs a Rust-side fix.
export const ChangePermissionModeSchema = messageSchema('update_permissions', {
  mode: PermissionModeSchema,
});
export type ChangePermissionMode = z.infer<typeof ChangePermissionModeSchema>;

// 8. toggle_context_file — add/remove context file
export const ToggleContextFileSchema = messageSchema('toggle_context_file', {
  path: z.string(),
  action: z.enum(['add', 'remove']),
});
export type ToggleContextFile = z.infer<typeof ToggleContextFileSchema>;

// 9. change_routing — change model routing for a category
export const ChangeRoutingSchema = messageSchema('change_routing', {
  category: ActionCategorySchema,
  tier: z.enum(['fast', 'balanced', 'capable']),
});
export type ChangeRouting = z.infer<typeof ChangeRoutingSchema>;

// 10. clear_chat — Ctrl+L
export const ClearChatSchema = messageSchema('clear_chat', {});
export type ClearChat = z.infer<typeof ClearChatSchema>;

// 11. slash_command — user issued a slash command
// NOTE: Rust Action::SlashCommand(String) is a newtype variant.
// Internally tagged serde cannot deserialize newtype(String).
// This is a KNOWN Rust-side limitation — needs Rust fix to use struct variant.
export const SlashCommandSchema = messageSchema('slash_command', {
  command: z.string(),
});
export type SlashCommand = z.infer<typeof SlashCommandSchema>;

// 12. update_model — change default model
// NOTE: Rust Action::UpdateModel(String) is a newtype variant.
// Internally tagged serde cannot deserialize newtype(String).
// This is a KNOWN Rust-side limitation — needs Rust fix to use struct variant.
export const UpdateModelSchema = messageSchema('update_model', {
  model: z.string(),
});
export type UpdateModel = z.infer<typeof UpdateModelSchema>;

// 13. moe_dispatch — parallel agent dispatch
export const MoeDispatchSchema = messageSchema('moe_dispatch', {
  commands: z.array(z.string()),
});
export type MoeDispatch = z.infer<typeof MoeDispatchSchema>;

// 14. inject_skill — mid-task skill injection
export const InjectSkillSchema = messageSchema('inject_skill', {
  command: z.string(),
});
export type InjectSkill = z.infer<typeof InjectSkillSchema>;

// 15. voice_transcribed — voice transcription completed
// NOTE: Rust Action::VoiceTranscribed { text } — handled in TUI event loop.
export const VoiceTranscribedSchema = messageSchema('voice_transcribed', {
  text: z.string(),
});
export type VoiceTranscribed = z.infer<typeof VoiceTranscribedSchema>;

// 16. quit — user wants to exit
export const QuitSchema = messageSchema('quit', {});
export type Quit = z.infer<typeof QuitSchema>;

// ═══════════════════════════════════════════════════════════════════════════
// Discriminated union of ALL TUI actions
// ═══════════════════════════════════════════════════════════════════════════

export const TuiActionSchema = z.discriminatedUnion('type', [
  SubmitPromptSchema,
  RunInBackgroundSchema,
  CancelAgentSchema,
  ResolvePermissionSchema,
  ResolveAskUserSchema,
  KbFeedbackSchema,
  ChangePermissionModeSchema,
  ToggleContextFileSchema,
  ChangeRoutingSchema,
  ClearChatSchema,
  SlashCommandSchema,
  UpdateModelSchema,
  MoeDispatchSchema,
  InjectSkillSchema,
  VoiceTranscribedSchema,
  QuitSchema,
]);
export type TuiAction = z.infer<typeof TuiActionSchema>;

// ═══════════════════════════════════════════════════════════════════════════
// Any protocol message (either direction)
// ═══════════════════════════════════════════════════════════════════════════

export const AnyMessageSchema = z.union([EngineEventSchema, TuiActionSchema]);
export type AnyMessage = z.infer<typeof AnyMessageSchema>;
