# Operator Runbooks

These procedures assume you are in the repo root on the local Mac.

## Prerequisites

- a saved profile such as `profiles/arjun-lambda-mega.env.local`
- SSH access to the GPU host
- `MEGA_EMAIL` in the profile
- `MEGA_PASSWORD` exported locally if the remote host is not already logged into
  MEGA

Example:

```bash
export MEGA_PASSWORD="your-password"
```

## Run One Folder

Preferred path:

```bash
bash tools/run_saved_profile.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --mega-source-path "/Course Folder"
```

From a browser folder URL:

```bash
bash tools/run_saved_profile.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --mega-browser-folder-url "https://mega.nz/fm/QIhhnDha"
```

Manual direct-launch form:

```bash
bash tools/run_on_lambda_host.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --host 192.222.50.45 \
  --user ubuntu \
  --ssh-key /Users/arjun/.ssh/codex_lambda_20260329 \
  --mega-source-path "/Course Folder" \
  --git-ref "$(git rev-parse HEAD)" \
  --git-repo-url "$(git remote get-url origin)" \
  --local-output-dir transcriptions
```

## Monitor The GPU Host

Use the saved profile and pick the view you want:

```bash
bash tools/monitor_lambda_host.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --mode split
```

Other useful modes:

```bash
bash tools/monitor_lambda_host.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --mode gpu

bash tools/monitor_lambda_host.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --mode system

bash tools/monitor_lambda_host.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --mode dashboard

bash tools/monitor_lambda_host.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --mode watch \
  --interval 1

bash tools/monitor_lambda_host.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --mode snapshot
```

## Run The Latest N Pending Folders

Refresh the catalog first:

```bash
python3 tools/catalog_jobs.py \
  --profile profiles/arjun-lambda-mega.env.local \
  sync-account
```

Inspect the newest pending top-level folders:

```bash
python3 tools/catalog_jobs.py \
  --db-path catalog/transcription_catalog.db \
  list-folders \
  --sort created_desc \
  --limit 30 \
  --top-level-only \
  --pending-transcription-only
```

Run them through the batch wrapper:

```bash
bash tools/run_catalog_batch.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --sort created_desc \
  --limit 30 \
  --top-level-only \
  --pending-transcription-only
```

Use a date window when you want a specific slice:

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

## Resume After SSH Or Terminal Drop

First, check whether the remote run is still active:

```bash
ssh -i /Users/arjun/.ssh/codex_lambda_20260329 ubuntu@192.222.50.45 \
  "pgrep -af '[t]ranscribe_mega_folder.py'"
```

If the remote run is still active:

- do not launch a second run against the same source
- wait for the manifest to appear in the local run directory or recover it after
  the run ends
- use transcript sync commands to pull in completed `.txt` files while the job
  is still progressing

If the remote run has ended and the local session died, recover local state with
the catalog:

```bash
python3 tools/catalog_jobs.py \
  --profile profiles/arjun-lambda-mega.env.local \
  --db-path catalog/transcription_catalog.db \
  sync-source \
  --source-path "/Course Folder"

python3 tools/catalog_jobs.py \
  --profile profiles/arjun-lambda-mega.env.local \
  --db-path catalog/transcription_catalog.db \
  sync-transcripts \
  --folder-path "/Course Folder" \
  --output-dir transcriptions/_mega_mirror
```

## Recover A Manifest After An Interrupted Run

If the manifest already exists locally, ingest it directly:

```bash
python3 tools/catalog_jobs.py \
  --profile profiles/arjun-lambda-mega.env.local \
  --db-path catalog/transcription_catalog.db \
  ingest-run \
  --run-id 20260330-031317 \
  --manifest-path "transcriptions/Course Folder/20260330-031317/run-manifest-20260330-031317.json"
```

If the run truly failed and there is no recoverable manifest, record the failure:

```bash
python3 tools/catalog_jobs.py \
  --profile profiles/arjun-lambda-mega.env.local \
  --db-path catalog/transcription_catalog.db \
  mark-run-failed \
  --run-id 20260330-031317 \
  --source-path "/Course Folder" \
  --error-text "local session ended before manifest recovery"
```

Then refresh the source view:

```bash
python3 tools/catalog_jobs.py \
  --profile profiles/arjun-lambda-mega.env.local \
  --db-path catalog/transcription_catalog.db \
  show-source "/Course Folder"
```

## Validate MEGA Transcript Presence Vs Local Mirror

Check that processed transcripts still exist remotely and are mirrored locally:

```bash
python3 tools/catalog_jobs.py \
  --profile profiles/arjun-lambda-mega.env.local \
  --db-path catalog/transcription_catalog.db \
  verify-transcripts \
  --folder-path "/Course Folder" \
  --output-dir transcriptions/_mega_mirror
```

Use `--validate-content` when you want a stronger check:

```bash
python3 tools/catalog_jobs.py \
  --profile profiles/arjun-lambda-mega.env.local \
  --db-path catalog/transcription_catalog.db \
  verify-transcripts \
  --folder-path "/Course Folder" \
  --output-dir transcriptions/_mega_mirror \
  --validate-content
```

## Backfill Missing Local Mirror Files

Backfill one folder:

```bash
python3 tools/catalog_jobs.py \
  --profile profiles/arjun-lambda-mega.env.local \
  --db-path catalog/transcription_catalog.db \
  sync-transcripts \
  --folder-path "/Course Folder" \
  --output-dir transcriptions/_mega_mirror
```

Backfill everything currently marked as processed:

```bash
python3 tools/catalog_jobs.py \
  --profile profiles/arjun-lambda-mega.env.local \
  --db-path catalog/transcription_catalog.db \
  sync-transcripts \
  --output-dir transcriptions/_mega_mirror
```

Use `--force` only when you want to overwrite the local mirror copy.

## Inspect Partial Failures And Retry Safely

Summarize recent failure patterns:

```bash
python3 tools/catalog_jobs.py \
  --db-path catalog/transcription_catalog.db \
  analyze-failures
```

Inspect one folder:

```bash
python3 tools/catalog_jobs.py \
  --db-path catalog/transcription_catalog.db \
  show-source "/Course Folder"
```

Retry the folder safely without reprocessing existing transcripts:

```bash
bash tools/run_saved_profile.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --mega-source-path "/Course Folder"
```

Use `--force` only if you intentionally want to regenerate transcripts that are
already present.

For debugging or canary runs where you do not want automatic folder rename:

```bash
bash tools/run_saved_profile.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --mega-source-path "/Course Folder" \
  --disable-source-rename
```

## Troubleshooting

### Stale `mega-cmd-server`

Symptom:
- `mega-whoami` or `mega-ls` behaves inconsistently, hangs, or returns startup
  chatter instead of normal output

What to do:
- rerun the launcher first; it already tries to restart and revalidate the MEGA
  command server
- if you are debugging manually on the remote host, clear the stale server
  before retrying

Manual reset:

```bash
ssh -i /Users/arjun/.ssh/codex_lambda_20260329 ubuntu@192.222.50.45 \
  "pkill -f 'mega-cmd-server|megacmdserver' || true"
```

### `mega-get` Write Error

Symptom:
- file downloads fail with `Write error`

What to do:
- retry the folder in a lower-concurrency mode first

```bash
bash tools/run_saved_profile.sh \
  --profile profiles/arjun-lambda-mega.env.local \
  --mega-source-path "/Course Folder" \
  --io-workers 1 \
  --transcribe-workers 1
```

Do not start a second concurrent run for the same folder while debugging this.

### Remote Environment Drift

Symptom:
- whisper backend mismatch, import errors, or behavior changes after a deploy

What to do:
- rerun through the launcher so the runtime bootstrap can refresh the pinned
  environment for the selected Git ref
- confirm the Git ref and Git repo URL in the profile are what you expect

### Catalog Path Drift After Rename

Symptom:
- the source folder was renamed to `*_transcript_done`, but an old path still
  appears in local inspection

What to do:

```bash
python3 tools/catalog_jobs.py \
  --profile profiles/arjun-lambda-mega.env.local \
  --db-path catalog/transcription_catalog.db \
  sync-account
```

Then re-check:

```bash
python3 tools/catalog_jobs.py show-source "/Course Folder_transcript_done"
```

## Job Complete Means

Treat a job as complete only when all of the following are true:

- the catalog shows `transcript_done`
- the MEGA folder contains `.txt` outputs beside the source media
- the local per-run manifest exists
- the local mirror contains the expected transcript files
- the folder rename happened only if rename was enabled and the run fully
  succeeded
