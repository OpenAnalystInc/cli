# Transcription Orchestrator Skill Refresh Design

## Goal

Refresh the existing `transcription-pipeline-orchestrator` skill so it teaches
the current Lambda runtime workflow instead of the older "SSH in, find code,
and run it manually" process.

## Why

The current skill is out of sync with the repo:

- Lambda jobs now run through `tools/run_transcription_job.sh`
- code is fetched from GitHub by pinned ref on the Lambda side
- persistent Lambda filesystems are the reusable base
- private GitHub access is resolved on the Mac and forwarded to Lambda
- validation now centers on local artifacts plus `run-manifest-<run-id>.json`

If the skill stays on the old flow, it will push future runs toward fragile,
manual orchestration and miss the safer private-repo runtime path.

## Design

Update the existing skill in place.

- Keep it as the one skill for Lambda transcription orchestration.
- Change the description so it triggers on the current use cases rather than
  describing obsolete implementation details.
- Make the default operator path:
  - validate local prerequisites on the Mac
  - ensure the Git ref is committed and pushed
  - resolve private GitHub auth locally when needed
  - launch one Lambda instance per job with `tools/run_transcription_job.sh`
  - reuse the Lambda filesystem and pinned runtime cache
  - validate manifest, downloads, and run status locally
- Keep `tools/run_saved_profile.sh` and `tools/run_on_lambda_host.sh` as
  fallback paths for cases where the instance is already available or the user
  wants a lower-level entrypoint.

## Validation Rules

The refreshed skill should explicitly require:

- never printing secrets
- failing locally when a private GitHub token is required but unavailable
- checking the run manifest before claiming success
- reporting whether the instance was terminated or intentionally kept

## Expected Outcome

Future sessions that use the skill should follow the repo's real operational
flow for public or private GitHub repos, and should validate the job outcome
using the local artifact directory instead of ad hoc remote inspection.
