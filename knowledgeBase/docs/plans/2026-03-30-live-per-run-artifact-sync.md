# Live Per-Run Artifact Sync Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make `tools/run_on_lambda_host.sh` copy remote run `downloads/` and
finished `exports/` into the local per-run folder while a job is still running.

**Architecture:** Reuse the existing incremental sync loop instead of creating a
second background worker. Each loop iteration should continue syncing
transcripts to `_mega_mirror` and also best-effort copy remote per-run
artifacts into the local run directory. Final manifest recovery stays unchanged.

**Tech Stack:** Bash launcher logic, Python unittest harness, fake `ssh`/`scp`
test doubles.

---

### Task 1: Add the failing launcher test

**Files:**
- Modify: `tests/test_run_on_lambda_host.py`
- Test: `tests/test_run_on_lambda_host.py`

**Step 1: Write the failing test**

Add a test that runs the launcher with:
- no recoverable manifest
- one active-run poll
- a brief fake remote execute delay

Assert that:
- the run still fails because the manifest never appears
- the per-run local `downloads/` folder receives a live-copied file
- the per-run local `exports/` folder receives a live-copied file

**Step 2: Run test to verify it fails**

Run: `python3 -m unittest tests.test_run_on_lambda_host.RunOnLambdaHostTests.test_live_sync_updates_per_run_downloads_and_exports_while_run_is_active`

Expected: FAIL because the launcher does not yet copy live per-run exports.

### Task 2: Extend the fake transport harness only as needed

**Files:**
- Modify: `tests/test_run_on_lambda_host.py`
- Test: `tests/test_run_on_lambda_host.py`

**Step 1: Add minimal fake behavior**

Teach the fake `ssh`/`scp` helpers to:
- delay the fake execute-run path when requested
- materialize simple placeholder files for copied remote `downloads/` and
  `exports/` trees

**Step 2: Re-run the same test**

Run: `python3 -m unittest tests.test_run_on_lambda_host.RunOnLambdaHostTests.test_live_sync_updates_per_run_downloads_and_exports_while_run_is_active`

Expected: Still FAIL until production code is updated.

### Task 3: Implement live per-run artifact sync in the launcher

**Files:**
- Modify: `tools/run_on_lambda_host.sh`
- Test: `tests/test_run_on_lambda_host.py`

**Step 1: Add a local per-run exports path**

Define a `LOCAL_EXPORT_DIR` under the local run artifact directory.

**Step 2: Add best-effort copy helpers**

Add small helpers that copy:
- remote `downloads/` to the local per-run `downloads/`
- remote `exports/` to the local per-run `exports/`

These helpers should swallow copy failures so active runs keep going.

**Step 3: Wire the helpers into the incremental sync loop**

Make each incremental pass:
- sync transcripts to `_mega_mirror`
- copy remote downloads
- copy remote exports

**Step 4: Keep final recovery behavior intact**

Do not remove the existing final manifest/download recovery path.

**Step 5: Run the targeted test**

Run: `python3 -m unittest tests.test_run_on_lambda_host.RunOnLambdaHostTests.test_live_sync_updates_per_run_downloads_and_exports_while_run_is_active`

Expected: PASS

### Task 4: Guard against regressions

**Files:**
- Modify: `tests/test_run_on_lambda_host.py` only if expectation updates are needed
- Test: `tests/test_run_on_lambda_host.py`

**Step 1: Run focused launcher coverage**

Run: `python3 -m unittest tests.test_run_on_lambda_host`

Expected: PASS

**Step 2: Run adjacent launcher tests if needed**

Run: `python3 -m unittest tests.test_run_saved_profile`

Expected: PASS

### Task 5: Update docs

**Files:**
- Modify: `docs/TRANSCRIPTION_PROCESS.md`

**Step 1: Document the new live per-run behavior**

Update the local mirror/per-run artifacts section so it explains that active
runs now live-update both:
- `_mega_mirror`
- per-run `downloads/` and `exports/`

**Step 2: Re-read the wording for operator clarity**

Keep the docs focused on where to look during an active run versus after
manifest recovery.
