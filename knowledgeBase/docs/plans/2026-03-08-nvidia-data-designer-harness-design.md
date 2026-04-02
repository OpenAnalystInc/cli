# NVIDIA Data Designer Harness Design

**Date:** 2026-03-08

**Goal:** Add a small local Python CLI to this repository that stores an NVIDIA NeMo Data Designer managed-service API key in a local untracked file, loads JSON scenario fixtures, and runs one scenario at a time against the hosted `preview` API.

## Approved Decisions

- Use the hosted NVIDIA managed service, not the local NeMo Data Designer codebase.
- Implement a Python CLI rather than a shell-first harness.
- Store the API key in a local file under `assets/`.
- Keep the key file untracked.
- Model the live scenarios on NVIDIA's managed-service tutorials.
- Start with two scenarios only:
  - `retail_reviews_basic`
  - `patient_notes_seeded`

## External Reference Shape

The NVIDIA managed-service tutorials in `NVIDIA/GenerativeAIExamples` connect to:

- Base URL: `https://ai.api.nvidia.com/v1/nemo/dd`
- Auth: `Authorization: Bearer <api-key>`
- Client: `NeMoDataDesignerClient`
- Builder: `DataDesignerConfigBuilder`
- Execution path: `preview(config_builder, num_records=...)`

The managed-service examples use the `nemo-microservices[data-designer]` SDK and configure:

- one model config with provider `nvidiabuild`
- sampler columns
- expression columns
- `llm-text` columns
- optional seed datasets for preview jobs

## Repository Changes

Planned files:

- Create `tools/nvidia_data_designer.py`
- Create `fixtures/nvidia-data-designer/retail_reviews_basic.json`
- Create `fixtures/nvidia-data-designer/patient_notes_seeded.json`
- Create `tests/test_nvidia_data_designer.py`
- Create `requirements-nvidia-data-designer.txt`
- Modify or create `.gitignore`
- Create `output/nvidia-data-designer/` at runtime

Credential storage:

- Save the API key in `assets/nvidia_data_designer_api_key.txt`
- Do not print the key after saving it
- Treat a missing or empty key file as a hard error

## CLI Design

The CLI should support these commands:

- `list-scenarios`
  - Enumerate available JSON fixtures by basename.
- `save-key --key <value>`
  - Save the NVIDIA key to `assets/nvidia_data_designer_api_key.txt`
  - Apply restrictive permissions where the platform allows it
- `run <scenario>`
  - Load the saved key
  - Load the fixture
  - Build the NVIDIA SDK config
  - Execute `preview(...)`
  - Write a timestamped artifact to `output/nvidia-data-designer/`

Expected runtime output should stay short:

- scenario name
- status
- record count
- artifact path

## Fixture Schema

The JSON fixtures should stay intentionally narrow in v1 so the script is predictable and maintainable.

Top-level fields:

- `name`
- `description`
- `preview`
  - `num_records`
- `model`
  - `alias`
  - `provider`
  - `model`
  - `inference_parameters`
- optional `seed_dataset`
- `columns`

Supported column families in v1:

- `sampler`
- `expression`
- `llm-text`

The CLI should reject unsupported column types rather than guessing.

## Scenario Definitions

### 1. `retail_reviews_basic`

Purpose:

- exercise the simplest hosted preview flow end-to-end
- verify sampler columns and one `llm-text` generation column

Shape:

- category and subcategory samplers
- person sampler
- rating sampler
- one review text generation column

Expected value:

- confirms the basic managed-service path works with the saved key
- gives a fast smoke test with no external seed data dependency

### 2. `patient_notes_seeded`

Purpose:

- verify the hosted preview flow can use a seed dataset and produce conditioned text output

Shape:

- Hugging Face dataset seed reference:
  - `gretelai/symptom_to_diagnosis/train.jsonl`
- patient and doctor samplers
- expression columns for identifiers and names
- one physician-notes `llm-text` column

Expected value:

- confirms the service can handle seeded generation, not just fully synthetic rows

## Runtime Data Flow

1. User saves the key with `save-key`.
2. CLI loads the requested scenario fixture.
3. CLI constructs `NeMoDataDesignerClient` with:
   - base URL `https://ai.api.nvidia.com/v1/nemo/dd`
   - bearer token header from the saved key
4. CLI builds `DataDesignerConfigBuilder`.
5. CLI applies:
   - model config
   - optional seed dataset
   - configured columns
6. CLI calls `validate()`.
7. CLI calls `preview(...)`.
8. CLI writes a timestamped artifact containing:
   - scenario metadata
   - request summary
   - sample record if present
   - row count if derivable
   - raw preview serialization where feasible

## Guardrails

- Never print the full API key.
- Never commit the key file.
- Do not implement `run-all` in v1.
- Do not try to support every Data Designer column type in v1.
- Fail fast on malformed fixtures, missing SDK imports, or unsupported fixture fields.
- Surface NVIDIA API errors clearly without leaking credentials.

## Verification Plan

Unit-level verification:

- scenario discovery works
- saved key is persisted and reloaded correctly
- fixture parsing rejects bad input
- config builder wiring handles both the basic and seeded scenario shapes
- `run` writes an output artifact

Live verification:

- `python tools/nvidia_data_designer.py list-scenarios`
- `python tools/nvidia_data_designer.py save-key --key '<redacted>'`
- `python tools/nvidia_data_designer.py run retail_reviews_basic`
- `python tools/nvidia_data_designer.py run patient_notes_seeded`

Success criteria:

- both scenarios return preview data from the hosted NVIDIA service
- artifacts are written locally
- no credential material is echoed to stdout

## Constraints And Notes

- This workspace is currently not a Git repository, so the design document cannot be committed here yet.
- The implementation should use TDD for code changes, but the live NVIDIA calls are best treated as manual integration verification rather than default automated tests.
