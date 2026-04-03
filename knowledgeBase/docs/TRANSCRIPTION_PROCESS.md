# Transcription Process

This is the current end-to-end operator flow for MEGA transcription.

## 1. Source Selection

The launchers accept exactly one source form:

- `--mega-source-path "/Folder/Subfolder"`
- `--mega-browser-folder-url "https://mega.nz/fm/..."`
- `--mega-source-link "https://mega.nz/folder/..."`

For browser folder URLs, the local catalog resolves the browser locator to a
canonical MEGA path before the remote run starts.

## 2. Catalog Preparation

Before the remote job starts, the local tooling:

- initializes `catalog/transcription_catalog.db` on first use
- resolves browser-folder URLs when needed
- refreshes the selected source subtree with `sync-source`
- creates a `job_runs` row for the new `run_id`

This makes the local catalog the operator’s control plane for status and
recovery.

## 3. Remote Bootstrap

The launcher connects to the existing GPU host over SSH and prepares a pinned
runtime:

- bootstraps the repo from the requested Git ref
- reuses or refreshes the Python environment based on dependency fingerprinting
- ensures `MEGAcmd` is installed and the command server is healthy
- reuses an existing remote MEGA session or logs in with `MEGA_EMAIL` and
  `MEGA_PASSWORD` when needed

The preferred operator entrypoint is `tools/run_saved_profile.sh`. The direct
launcher `tools/run_on_lambda_host.sh` is for manual overrides and debugging.

## 4. Remote Download And Transcription

The remote runner:

- enumerates processable files from the MEGA source
- downloads source files with `mega-get`
- stores run-local downloads under the remote run directory
- transcribes media with `faster-whisper` when the CUDA backend is available
- falls back to `torch` + `openai-whisper` when the host GPU runtime cannot use
  the `ctranslate2` CUDA path

The system is designed to keep the job moving even when the preferred whisper
backend is unavailable on the GPU host.

## 5. Upload And Folder Finalization

For folder sources, successful output goes back into the same MEGA folder:

- `Lesson 01.mp4` becomes `Lesson 01.txt` in the same MEGA directory
- existing `.txt` transcripts cause `skipped_existing`, not duplicate output
- folder rename to `*_transcript_done` happens only when every processable file
  is either `processed` or `skipped_existing`
- if any file fails, the folder remains in place and the run ends as
  `partial_failed`

If `--disable-source-rename` is set, the run still uploads transcripts in
place, but skips the rename step.

## 6. Manifest Recovery And Catalog Ingestion

After the remote command exits, the launcher:

- attempts to recover the remote run manifest even if the SSH session was noisy
  or partially interrupted
- copies the remote artifact directory back to the local machine
- copies remote downloaded source files into the local per-run downloads folder
- ingests the manifest into the catalog
- refreshes the source subtree after rename or completion

This is what turns a raw remote run into stable local state that operators can
inspect and retry safely.

## 7. Local Mirror Behavior

The local transcript mirror lives under:

```text
transcriptions/_mega_mirror/<remote-folder>/...
```

Mirror behavior is split into two parts:

- incremental sync during active runs for the tracked source folder
- incremental live copies of remote per-run `downloads/` and finished
  `exports/` into the local run folder
- a final sync after manifest recovery so the local mirror catches up with the
  completed or renamed MEGA folder path

Per-run artifacts still live separately:

```text
transcriptions/<source-label>/<run-id>/
```

That directory is for manifests, live-copied per-run downloads and exports,
copied remote downloads after recovery, and run-local artifacts. The
`_mega_mirror` tree is the stable transcript destination for local browsing and
later search.

## 8. Operator Verification

An operator should verify:

- the source folder status in `catalog/transcription_catalog.db`
- the MEGA folder contains `.txt` files beside the media
- the local mirror contains the same completed transcripts
- the run manifest exists locally
- the folder rename happened only when the run fully succeeded

The easiest follow-up commands are:

```bash
python3 tools/catalog_jobs.py show-source "/Course Folder"
python3 tools/catalog_jobs.py --profile profiles/arjun-lambda-mega.env.local --db-path catalog/transcription_catalog.db verify-transcripts --folder-path "/Course Folder" --output-dir transcriptions/_mega_mirror
python3 tools/catalog_jobs.py --profile profiles/arjun-lambda-mega.env.local --db-path catalog/transcription_catalog.db sync-transcripts --folder-path "/Course Folder" --output-dir transcriptions/_mega_mirror
```

## 9. Optional S4 Context

`MEGA S4` is optional in the current system. If configured, it can be used for
retained artifacts or staging, but the primary operator workflow does not depend
on it. The default documentation and runbooks assume MEGA-only operation unless
the operator deliberately enables S4-related flags in the profile or launcher.
