# Transcription Objectives

This document defines what the current transcription system is supposed to do,
what counts as success, and where the operator should look for state.

## Primary Objectives

- Transcribe supported media files from MEGA folders on an existing GPU host.
- Upload `.txt` transcripts into the same MEGA folder as the source media.
- Rename the source folder to `*_transcript_done` only when the whole folder is
  complete.
- Keep a local searchable catalog of folders, files, statuses, and run history.
- Keep a stable local transcript mirror under `transcriptions/_mega_mirror`.

## Operating Principles

- `MEGA` is the primary source of truth for source media and final transcript
  placement.
- `catalog/transcription_catalog.db` is the local source of truth for job,
  folder, file, and run status.
- Local mirrors make downstream browsing, validation, and search faster, but
  they do not replace the MEGA-side transcript destination.
- The default operator path is MEGA-first. Optional `MEGA S4` support exists as
  secondary or legacy behavior, not as a mandatory dependency.

## Source Of Truth By Artifact

- Source media:
  MEGA folder or resolved browser-folder source
- Final transcript placement:
  the same MEGA folder as the source media
- Run manifest and per-run downloads:
  `transcriptions/<source-label>/<run-id>/`
- Stable local transcript mirror:
  `transcriptions/_mega_mirror/<remote-folder>/...`
- Local status and search:
  `catalog/transcription_catalog.db`

## Status Model

### Folder statuses

- `not_started`: folder is cataloged but no processable files have been attempted yet
- `in_progress`: at least one file has been processed but pending files remain
- `running`: an active run is in progress
- `partial_failed`: at least one processable file failed in the most recent run
- `transcript_done`: all processable files are complete or already had a usable
  transcript
- `needs_source_resolution`: a `mega.nz/fm/...` browser locator has not yet been
  mapped to a canonical MEGA path
- `stale`: catalog bookkeeping marker for data that should be refreshed

### File statuses

- `pending`: no completed transcript exists yet
- `processed`: transcript was generated in a run
- `skipped_existing`: transcript already existed, so the file was not
  reprocessed
- `failed`: the latest run failed for that file

## Definition Of Done

A folder should be treated as complete only when all of the following are true:

- the catalog shows the folder as `transcript_done`
- the processed media files have matching `.txt` files in MEGA
- the local run manifest exists under `transcriptions/<source-label>/<run-id>/`
- the local transcript mirror contains the completed transcripts under
  `transcriptions/_mega_mirror`
- if rename is enabled, the folder path ends in `*_transcript_done`

If any processable file failed, the folder is not done even if some transcripts
were uploaded successfully.

## Non-Goals

- Storing transcript text in the catalog itself as the primary artifact
- Treating S4 staging as mandatory for all runs
- Renaming folders when there are still pending or failed processable files
- Using the local mirror as a substitute for MEGA-side transcript placement
