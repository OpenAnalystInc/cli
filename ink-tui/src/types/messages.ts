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
export const StreamDeltaSchema = messageSchema('stream_delta', {
  agentId: z.string(),
  content: z.string(),
  done: z.boolean(),
});
export type StreamDelta = z.infer<typeof StreamDeltaSchema>;

// 2. stream_end — assistant finished streaming
export const StreamEndSchema = messageSchema('stream_end', {
  agentId: z.string(),
});
export type StreamEnd = z.infer<typeof StreamEndSchema>;

// 3. tool_call_start — tool execution begins
export const ToolCallStartSchema = messageSchema('tool_call_start', {
  agentId: z.string(),
  toolId: z.string(),
  toolName: z.string(),
  inputPreview: z.string(),
});
export type ToolCallStart = z.infer<typeof ToolCallStartSchema>;

// 4. tool_call_update — tool progress (partial output)
export const ToolCallUpdateSchema = messageSchema('tool_call_update', {
  agentId: z.string(),
  toolId: z.string(),
  output: z.string(),
});
export type ToolCallUpdate = z.infer<typeof ToolCallUpdateSchema>;

// 5. tool_call_complete — tool finished
export const ToolCallCompleteSchema = messageSchema('tool_call_complete', {
  agentId: z.string(),
  toolId: z.string(),
  status: z.enum(['completed', 'failed']),
  output: z.string(),
  durationMs: z.number(),
  diff: DiffInfoSchema.optional(),
});
export type ToolCallComplete = z.infer<typeof ToolCallCompleteSchema>;

// 6. permission_request — needs user approval
export const PermissionRequestSchema = messageSchema('permission_request', {
  requestId: z.string(),
  agentId: z.string(),
  toolName: z.string(),
  toolInput: z.string(),
  requiredMode: PermissionModeSchema,
  filePath: z.string().optional(),
  description: z.string().optional(),
});
export type PermissionRequest = z.infer<typeof PermissionRequestSchema>;

// 7. ask_user_request — needs user choice or text input
export const AskUserRequestSchema = messageSchema('ask_user_request', {
  requestId: z.string(),
  agentId: z.string(),
  question: z.string(),
  options: z.array(z.string()).optional(),
  defaultValue: z.string().optional(),
  allowFreeText: z.boolean(),
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

// 14. kb_result — knowledge base query result
export const KbResultSchema = messageSchema('kb_result', {
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
  ToolCallCompleteSchema,
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

// 4. resolve_permission — answer permission dialog
export const ResolvePermissionSchema = messageSchema('resolve_permission', {
  requestId: z.string(),
  decision: z.enum(['allow', 'deny']),
});
export type ResolvePermission = z.infer<typeof ResolvePermissionSchema>;

// 5. resolve_ask_user — answer ask-user dialog
export const ResolveAskUserSchema = messageSchema('resolve_ask_user', {
  requestId: z.string(),
  answer: z.string(),
});
export type ResolveAskUser = z.infer<typeof ResolveAskUserSchema>;

// 6. kb_feedback — rate KB result
export const KbFeedbackSchema = messageSchema('kb_feedback', {
  queryId: z.number(),
  rating: z.enum(['positive', 'negative', 'corrected']),
  comment: z.string().optional(),
  correction: z.string().optional(),
});
export type KbFeedback = z.infer<typeof KbFeedbackSchema>;

// 7. change_permission_mode — Ctrl+P mode cycle
export const ChangePermissionModeSchema = messageSchema('change_permission_mode', {
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
export const SlashCommandSchema = messageSchema('slash_command', {
  command: z.string(),
});
export type SlashCommand = z.infer<typeof SlashCommandSchema>;

// 12. update_model — change default model
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

// 15. quit — user wants to exit
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
  QuitSchema,
]);
export type TuiAction = z.infer<typeof TuiActionSchema>;

// ═══════════════════════════════════════════════════════════════════════════
// Any protocol message (either direction)
// ═══════════════════════════════════════════════════════════════════════════

export const AnyMessageSchema = z.union([EngineEventSchema, TuiActionSchema]);
export type AnyMessage = z.infer<typeof AnyMessageSchema>;
