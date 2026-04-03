---
name: transcription-pipeline-orchestrator
description: >
  Runs in-place batch transcription from MEGA folder paths, resolved
  mega.nz/fm browser folders, or true public export links on an existing
  SSH-accessible GPU host using MEGAcmd, whisper-based transcription, and a
  local SQLite catalog. Manages a multi-host GPU fleet with automated
  scheduling, monitoring, and OCR processing.
trigger_phrases:
  - "transcribe"
  - "run transcription"
  - "transcription pipeline"
  - "process audio"
  - "process video"
  - "start transcription job"
  - "launch transcription"
  - "mega transcription"
  - "gpu fleet"
  - "gpu utilization"
  - "scheduler"
  - "check workers"
  - "fleet status"
---

# In-Place MEGA Transcription Skill

Use this skill when the user wants to transcribe media from MEGA on a remote GPU
host, upload transcripts back into the same MEGA folder, and maintain a local
SQLite catalog of folders, files, and job status.

## Default Operator Model

- Saved-profile launcher: `tools/run_saved_profile.sh`
- Direct launcher: `tools/run_on_lambda_host.sh`
- Catalog batch launcher: `tools/run_catalog_batch.sh`
- Remote runner: `tools/transcribe_mega_folder.py`
- Local catalog: `tools/catalog_jobs.py`
- SQLite DB: `catalog/transcription_catalog.db`
- Source storage: MEGA Cloud Drive paths, resolved `mega.nz/fm/...` folders, or
  true public export links
- Transcript outputs: `.txt` files in the source folder
- Local mirror: `transcriptions/_mega_mirror`
- Per-run artifacts: `transcriptions/<source-label>/<run-id>/`

`MEGA S4` is optional and should not be treated as mandatory in the default
operator flow.

## Security Rules

1. Never print `MEGA_PASSWORD`, optional S4 credentials, or SSH private key contents in chat.
2. Refer to secrets by environment variable name whenever possible.
3. If verifying whether a secret exists, only report `YES` or `NO`.
4. Treat `mega.nz/fm/...` as a browser locator that must be resolved before the remote run.

## Required Inputs

Before starting, verify these are available:

```bash
echo "MEGA_EMAIL set:         $([ -n "$MEGA_EMAIL" ] && echo YES || echo NO)"
echo "MEGA_PASSWORD set:      $([ -n "$MEGA_PASSWORD" ] && echo YES || echo NO)"
echo "SSH key exists:         $([ -f "$SSH_KEY_PATH" ] && echo YES || echo NO)"
```

`MEGA_PASSWORD` is only required when the remote host is not already
authenticated with MEGA. Optional S4 credentials should only be checked if the
user explicitly wants S4-backed staging or retention.

## Core Flow

### 1. Confirm the source and launcher path

- Prefer the saved-profile launcher.
- Confirm exactly one source input:
  - `--mega-source-path`
  - `--mega-browser-folder-url`
  - `--mega-source-link`
- Confirm whether the user wants:
  - a single-folder run
  - a catalog-driven batch run
  - a recovery or retry action

### 2. Validate the host only when needed

Use a read-only SSH check if connectivity is uncertain:

```bash
ssh -i "$SSH_KEY_PATH" "$REMOTE_USER@$REMOTE_HOST" \
  'command -v python3 && command -v ffmpeg && nvidia-smi --query-gpu=name --format=csv,noheader'
```

### 3. Resolve browser folders when needed

If the user supplies `https://mega.nz/fm/...`, resolve it through the local
catalog:

```bash
python3 tools/catalog_jobs.py resolve-source \
  --profile profiles/arjun-lambda-mega.env.local \
  --mega-browser-folder-url "https://mega.nz/fm/QIhhnDha"
```

If resolution fails, the source should be stored as
`needs_source_resolution` and the run should not start.

### 4. Launch the run

Preferred launcher:

```bash
bash tools/run_saved_profile.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --mega-source-path "/Course Folder"
```

Direct launcher example:

```bash
bash tools/run_on_lambda_host.sh \
  --host "$REMOTE_HOST" \
  --user "$REMOTE_USER" \
  --ssh-key "$SSH_KEY_PATH" \
  --mega-source-path "$MEGA_SOURCE_PATH" \
  --git-ref "$GIT_REF" \
  --git-repo-url "$GIT_REPO_URL" \
  --local-output-dir "$LOCAL_OUTPUT_DIR"
```

Add these when needed:

- `--mega-source-link "$MEGA_SOURCE_LINK"`
- `--mega-output-path "$MEGA_OUTPUT_PATH"`
- `--model "$MODEL"`
- `--force`
- `--disable-source-rename`

### 5. What the launcher should do

The launcher should:

- initialize the SQLite catalog on first use with a full-account sync
- resolve `mega.nz/fm/...` URLs into canonical MEGA paths before the remote run
- refresh the selected source subtree before the run
- create a `job_runs` record before remote execution
- bootstrap the pinned runtime on the remote host
- install or refresh `MEGAcmd` if needed
- ensure the remote MEGA session is usable
- recover the run manifest even after a noisy SSH session
- ingest the returned manifest into SQLite after the run
- mirror transcripts locally under `transcriptions/_mega_mirror`

### 6. What the remote runner should do

The remote runner should:

- use MEGA folder enumeration for authenticated sources
- reject raw `mega.nz/fm/...` URLs as execution input
- download each source media file with `mega-get` into local scratch
- transcribe on GPU with `faster-whisper` when available
- fall back to `torch` + `openai-whisper` when the CUDA path is unavailable
- upload the `.txt` transcript back into the same MEGA folder as the media
- rename the source folder to `*_transcript_done` only when all supported media
  files are completed or skipped because transcripts already exist

### 7. Verify the result

After the run:

- verify local artifacts exist in `transcriptions/<source-label>/<run-id>/`
- verify local mirrored transcripts exist in `transcriptions/_mega_mirror/`
- verify the SQLite DB was updated
- verify the MEGA folder contains `.txt` files beside the media
- verify the source folder was renamed only on full success
- if failures exist in the manifest, report them explicitly

Helpful catalog commands:

```bash
python3 tools/catalog_jobs.py list-folders --db-path catalog/transcription_catalog.db
python3 tools/catalog_jobs.py list-folders --created-from 2026-03-01T00:00:00Z --created-to 2026-04-01T00:00:00Z --sort created_desc --limit 30 --top-level-only --pending-transcription-only
python3 tools/catalog_jobs.py list-files --folder-path "/Social Hacker – Surgical Pick Up 3.0" --sort created_desc
python3 tools/catalog_jobs.py show-source "/Social Hacker – Surgical Pick Up 3.0"
python3 tools/catalog_jobs.py search-files "Lesson 01" --kind media
python3 tools/catalog_jobs.py search-files "webpage" --kind image
```

For timestamp-driven queueing, use:

```bash
bash tools/run_catalog_batch.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --created-from 2026-03-01T00:00:00Z \
  --created-to 2026-04-01T00:00:00Z \
  --sort created_desc \
  --limit 30 \
  --top-level-only \
  --pending-transcription-only
```

## Fleet Operations

For GPU fleet management, scheduling, monitoring, and troubleshooting, see
[docs/FLEET_OPERATIONS.md](docs/FLEET_OPERATIONS.md).

Key operational commands:

```bash
# Check fleet status via SwiftBar plugin
bash ~/Library/SwiftBarPlugins/gpu-monitor.30s.sh

# Manual scheduler run
bash tools/cron_scheduler.sh

# Check a specific host
ssh -i $SSH_KEY_PATH ubuntu@$HOST 'nvidia-smi --query-gpu=memory.used,memory.total,utilization.gpu --format=csv,noheader; ps aux | grep transcribe_mega | grep -v grep | wc -l'

# Check pending backlog (active folders only)
python3 tools/catalog_jobs.py list-folders --db-path catalog/transcription_catalog.db --pending-transcription-only --top-level-only --limit 30
```

## Operational Notes

- Default output format is `.txt` only.
- Default model is `large-v3`.
- The MEGA account used on the GPU host must have write access to the source
  folder.
- Public export links still need an explicit `--mega-output-path`.
- This workflow reuses the existing GPU host instead of provisioning a new one.
- `MEGA S4` is optional and should be mentioned only when the user explicitly
  needs it.
- Fleet configuration lives in `profiles/workers.json`.
- Cron scheduler runs every 15 minutes in `fresh-only` mode.
- GH200s process all file kinds (media + OCR), A100s and V100s do media only.
- VRAM budget: ~5-6GB per whisper instance, ~10-15GB for marker OCR models.
