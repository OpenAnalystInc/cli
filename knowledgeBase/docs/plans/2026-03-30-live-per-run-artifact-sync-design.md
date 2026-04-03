# Live Per-Run Artifact Sync Design

## Goal

Make the local per-run folder update while a Lambda transcription job is still
running so operators can inspect remote `downloads/` and finished `exports/`
without waiting for manifest recovery.

## Current Behavior

`tools/run_on_lambda_host.sh` currently uses two local destinations:

- `transcriptions/_mega_mirror/...` for incremental transcript sync during an
  active run
- `transcriptions/<source>/<run-id>/...` for logs, manifest recovery, and final
  copied downloads after the remote command exits

That means the per-run folder often looks empty during an active run even though
remote progress is real.

## Chosen Approach

Extend the existing incremental sync loop in `tools/run_on_lambda_host.sh` so
each pass does three things:

1. keep the existing `sync-transcripts` call into `_mega_mirror`
2. copy remote run `downloads/` into the local per-run `downloads/` folder
3. copy remote run `exports/` into a new local per-run `exports/` folder

This keeps one loop, one stop mechanism, and one sync cadence.

## Why This Approach

- preserves the existing `_mega_mirror` behavior and operator workflow
- avoids a second background loop with separate lifecycle problems
- makes the per-run folder useful during active work, not only after recovery
- still keeps the final manifest-recovery path as the source of truth for
  completion and catalog ingestion

## Local Layout

During an active run, the launcher should populate:

- `transcriptions/<source>/<run-id>/downloads/...`
- `transcriptions/<source>/<run-id>/exports/...`
- `transcriptions/<source>/<run-id>/launcher-phases.log`
- `transcriptions/<source>/<run-id>/remote-preflight.log`
- `transcriptions/<source>/<run-id>/remote-exec.log`

The stable mirror remains:

- `transcriptions/_mega_mirror/<remote-folder>/...`

## Failure Handling

- If a live copy fails, the launcher should continue the run and retry on the
  next loop iteration.
- Final manifest recovery and the existing final copy steps remain unchanged so
  the run can still self-heal at the end.
- Missing manifests should still mark the run failed exactly as today.

## Tests

Add a launcher test that proves a run with no recoverable manifest still copies
live `downloads/` and `exports/` into the per-run folder while the job is
active. Keep the current recovery and `_mega_mirror` tests intact.
