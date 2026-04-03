# Transcription Orchestrator Skill Refresh Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Update the existing `transcription-pipeline-orchestrator` skill so it teaches the GitHub-pinned Lambda workflow with private GitHub token handling and manifest-based validation.

**Architecture:** Keep a single skill file, but replace the outdated remote-code-search workflow with the repo's current launcher flow. The skill should describe one recommended path through `tools/run_transcription_job.sh`, plus explicit fallback paths through the lower-level launchers.

**Tech Stack:** Markdown skill file, local shell launchers, Lambda Labs API helper scripts, GitHub token forwarding from macOS.

---

### Task 1: Record the operator design

**Files:**
- Create: `docs/plans/2026-03-30-transcription-orchestrator-skill-design.md`
- Create: `docs/plans/2026-03-30-transcription-orchestrator-skill-plan.md`

**Step 1: Write the design note**

Capture the current workflow, the reason for the refresh, the preferred
execution path, and the validation rules.

**Step 2: Verify the design note exists**

Run: `test -f docs/plans/2026-03-30-transcription-orchestrator-skill-design.md`
Expected: exit `0`

### Task 2: Rewrite the skill frontmatter and overview

**Files:**
- Modify: `/Users/arjun/.codex/skills/transcription-pipeline-orchestrator/SKILL.md`

**Step 1: Update the trigger description**

Replace the old description and remove unsupported frontmatter fields so the
skill is discoverable and valid for Codex skill loading.

**Step 2: Replace the obsolete overview**

Describe the new default flow:

- saved profile driven
- pinned Git refs
- Lambda filesystem reuse
- private GitHub auth from the Mac
- manifest-based validation

### Task 3: Replace the execution phases

**Files:**
- Modify: `/Users/arjun/.codex/skills/transcription-pipeline-orchestrator/SKILL.md`

**Step 1: Write the recommended path**

Document the sequence for:

- local prerequisite checks
- `git rev-parse` and `git ls-remote` verification
- private GitHub token resolution
- `tools/run_transcription_job.sh`
- `--keep-instance` handling

**Step 2: Add fallback paths**

Document when to use:

- `tools/run_saved_profile.sh`
- `tools/run_on_lambda_host.sh`

### Task 4: Add validation and failure handling

**Files:**
- Modify: `/Users/arjun/.codex/skills/transcription-pipeline-orchestrator/SKILL.md`

**Step 1: Document success verification**

Require inspection of:

- local artifact directory
- `run-manifest-<run-id>.json`
- manifest `status`
- summary counts

**Step 2: Document common failures**

Cover:

- missing Lambda credentials
- missing or unpushed Git ref
- missing private GitHub token
- failed bootstrap
- missing manifest

### Task 5: Verify the skill content matches the repo

**Files:**
- Modify: `/Users/arjun/.codex/skills/transcription-pipeline-orchestrator/SKILL.md`

**Step 1: Re-read current launch scripts**

Check:

- `tools/run_transcription_job.sh`
- `tools/run_saved_profile.sh`
- `tools/run_on_lambda_host.sh`
- `tools/bootstrap_transcription_runtime.sh`

**Step 2: Run syntax-free validation**

Run:

```bash
rg -n "run_transcription_job.sh|run_saved_profile.sh|run_on_lambda_host.sh|GITHUB_TOKEN_REQUIRED|run-manifest" /Users/arjun/.codex/skills/transcription-pipeline-orchestrator/SKILL.md
```

Expected: the refreshed skill references the current scripts and validation
artifacts directly.
