# NVIDIA Data Designer Harness Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a small Python CLI in this repository that stores an NVIDIA NeMo Data Designer API key locally, loads JSON fixtures for two managed-service preview scenarios, and writes run artifacts for each successful preview call.

**Architecture:** The implementation will use a single Python CLI entrypoint in `tools/nvidia_data_designer.py`, two JSON fixtures under `fixtures/nvidia-data-designer/`, and a small stdlib `unittest` suite for test-first development. Live preview calls will go through NVIDIA's `nemo-microservices[data-designer]` SDK against `https://ai.api.nvidia.com/v1/nemo/dd`, while unit tests will validate fixture parsing, key handling, and artifact writing without requiring a network call.

**Tech Stack:** Python 3, `argparse`, `json`, `pathlib`, `unittest`, `unittest.mock`, `nemo-microservices[data-designer]==1.5.0`

---

### Task 1: Add dependency and file scaffolding

**Files:**
- Create: `/Users/arjun/Documents/transcription-pipeline-skill/requirements-nvidia-data-designer.txt`
- Create: `/Users/arjun/Documents/transcription-pipeline-skill/fixtures/nvidia-data-designer/retail_reviews_basic.json`
- Create: `/Users/arjun/Documents/transcription-pipeline-skill/fixtures/nvidia-data-designer/patient_notes_seeded.json`
- Create or modify: `/Users/arjun/Documents/transcription-pipeline-skill/.gitignore`
- Test: `/Users/arjun/Documents/transcription-pipeline-skill/tests/test_nvidia_data_designer.py`

**Step 1: Write the failing test**

```python
import unittest
from pathlib import Path

from tools import nvidia_data_designer


class ScenarioDiscoveryTests(unittest.TestCase):
    def test_list_scenarios_returns_fixture_basenames(self):
        names = nvidia_data_designer.list_scenarios()
        self.assertIn("retail_reviews_basic", names)
        self.assertIn("patient_notes_seeded", names)
```

**Step 2: Run test to verify it fails**

Run: `python -m unittest tests.test_nvidia_data_designer.ScenarioDiscoveryTests.test_list_scenarios_returns_fixture_basenames -v`

Expected: FAIL with `ModuleNotFoundError` or `AttributeError` because the CLI module and fixtures do not exist yet.

**Step 3: Write minimal implementation**

```text
- add `nemo-microservices[data-designer]==1.5.0` to `requirements-nvidia-data-designer.txt`
- add `.gitignore` entries:
  - `assets/nvidia_data_designer_api_key.txt`
  - `output/nvidia-data-designer/`
  - `__pycache__/`
- create both fixture JSON files with `name`, `description`, `preview`, `model`, and `columns`
- create a minimal `tools/nvidia_data_designer.py` with a `list_scenarios()` function that reads `fixtures/nvidia-data-designer/*.json`
```

**Step 4: Run test to verify it passes**

Run: `python -m unittest tests.test_nvidia_data_designer.ScenarioDiscoveryTests.test_list_scenarios_returns_fixture_basenames -v`

Expected: PASS

**Step 5: Commit**

This directory is not currently a Git repository. If it is later attached to Git, commit these changes with:

```bash
git add .gitignore requirements-nvidia-data-designer.txt fixtures/nvidia-data-designer tests/test_nvidia_data_designer.py tools/nvidia_data_designer.py
git commit -m "feat: scaffold nvidia data designer harness"
```

### Task 2: Add credential save/load behavior

**Files:**
- Modify: `/Users/arjun/Documents/transcription-pipeline-skill/tools/nvidia_data_designer.py`
- Test: `/Users/arjun/Documents/transcription-pipeline-skill/tests/test_nvidia_data_designer.py`

**Step 1: Write the failing test**

```python
import tempfile
import unittest
from pathlib import Path

from tools import nvidia_data_designer


class CredentialTests(unittest.TestCase):
    def test_save_key_persists_trimmed_value_without_echoing(self):
        with tempfile.TemporaryDirectory() as tmpdir:
            key_path = Path(tmpdir) / "nvidia_key.txt"
            nvidia_data_designer.save_key("  secret-token  ", key_path=key_path)
            self.assertEqual(key_path.read_text().strip(), "secret-token")
```

**Step 2: Run test to verify it fails**

Run: `python -m unittest tests.test_nvidia_data_designer.CredentialTests.test_save_key_persists_trimmed_value_without_echoing -v`

Expected: FAIL because `save_key` does not exist yet.

**Step 3: Write minimal implementation**

```python
def default_key_path() -> Path:
    return Path(__file__).resolve().parent.parent / "assets" / "nvidia_data_designer_api_key.txt"


def save_key(value: str, key_path: Path | None = None) -> Path:
    cleaned = value.strip()
    if not cleaned:
        raise ValueError("API key cannot be empty.")
    destination = key_path or default_key_path()
    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_text(cleaned + "\n", encoding="utf-8")
    return destination


def load_key(key_path: Path | None = None) -> str:
    source = key_path or default_key_path()
    value = source.read_text(encoding="utf-8").strip()
    if not value:
        raise ValueError("Saved API key is empty.")
    return value
```

**Step 4: Run test to verify it passes**

Run: `python -m unittest tests.test_nvidia_data_designer.CredentialTests.test_save_key_persists_trimmed_value_without_echoing -v`

Expected: PASS

**Step 5: Commit**

```bash
git add tools/nvidia_data_designer.py tests/test_nvidia_data_designer.py
git commit -m "feat: add nvidia key persistence"
```

### Task 3: Add fixture loading and validation

**Files:**
- Modify: `/Users/arjun/Documents/transcription-pipeline-skill/tools/nvidia_data_designer.py`
- Test: `/Users/arjun/Documents/transcription-pipeline-skill/tests/test_nvidia_data_designer.py`

**Step 1: Write the failing test**

```python
import unittest

from tools import nvidia_data_designer


class FixtureValidationTests(unittest.TestCase):
    def test_load_scenario_rejects_unknown_column_type(self):
        bad_fixture = {
            "name": "bad",
            "preview": {"num_records": 1},
            "model": {"alias": "m", "provider": "nvidiabuild", "model": "x", "inference_parameters": {}},
            "columns": [{"name": "oops", "column_type": "structured-output"}],
        }
        with self.assertRaises(ValueError):
            nvidia_data_designer.validate_scenario(bad_fixture)
```

**Step 2: Run test to verify it fails**

Run: `python -m unittest tests.test_nvidia_data_designer.FixtureValidationTests.test_load_scenario_rejects_unknown_column_type -v`

Expected: FAIL because `validate_scenario` does not exist yet.

**Step 3: Write minimal implementation**

```python
SUPPORTED_COLUMN_TYPES = {"sampler", "expression", "llm-text"}


def validate_scenario(payload: dict) -> dict:
    required_top_level = {"name", "preview", "model", "columns"}
    missing = required_top_level.difference(payload)
    if missing:
        raise ValueError(f"Scenario is missing required fields: {sorted(missing)}")
    for column in payload["columns"]:
        column_type = column.get("column_type")
        if column_type not in SUPPORTED_COLUMN_TYPES:
            raise ValueError(f"Unsupported column type: {column_type}")
    return payload
```

**Step 4: Run test to verify it passes**

Run: `python -m unittest tests.test_nvidia_data_designer.FixtureValidationTests.test_load_scenario_rejects_unknown_column_type -v`

Expected: PASS

**Step 5: Commit**

```bash
git add tools/nvidia_data_designer.py tests/test_nvidia_data_designer.py
git commit -m "feat: validate nvidia scenario fixtures"
```

### Task 4: Map JSON fixtures into NVIDIA SDK config builders

**Files:**
- Modify: `/Users/arjun/Documents/transcription-pipeline-skill/tools/nvidia_data_designer.py`
- Test: `/Users/arjun/Documents/transcription-pipeline-skill/tests/test_nvidia_data_designer.py`

**Step 1: Write the failing test**

```python
import unittest

from tools import nvidia_data_designer


class BuilderMappingTests(unittest.TestCase):
    def test_build_request_spec_includes_seed_dataset_when_present(self):
        scenario = nvidia_data_designer.load_scenario("patient_notes_seeded")
        spec = nvidia_data_designer.build_request_spec(scenario)
        self.assertEqual(spec["seed_dataset"]["dataset"], "gretelai/symptom_to_diagnosis/train.jsonl")
        self.assertEqual(spec["preview"]["num_records"], 2)
```

**Step 2: Run test to verify it fails**

Run: `python -m unittest tests.test_nvidia_data_designer.BuilderMappingTests.test_build_request_spec_includes_seed_dataset_when_present -v`

Expected: FAIL because `build_request_spec` does not exist yet.

**Step 3: Write minimal implementation**

```python
def build_request_spec(scenario: dict) -> dict:
    spec = {
        "preview": scenario["preview"],
        "model": scenario["model"],
        "columns": scenario["columns"],
    }
    if "seed_dataset" in scenario:
        spec["seed_dataset"] = scenario["seed_dataset"]
    return spec


def build_config_builder(scenario: dict):
    from nemo_microservices.data_designer.essentials import (
        DataDesignerConfigBuilder,
        InferenceParameters,
        ModelConfig,
        SeedDatasetReference,
    )

    model = scenario["model"]
    builder = DataDesignerConfigBuilder(
        model_configs=[
            ModelConfig(
                alias=model["alias"],
                provider=model["provider"],
                model=model["model"],
                inference_parameters=InferenceParameters(**model["inference_parameters"]),
            )
        ]
    )
    if "seed_dataset" in scenario:
        seed = scenario["seed_dataset"]
        builder.with_seed_dataset(
            dataset_reference=SeedDatasetReference(
                dataset=seed["dataset"],
                datastore_settings=seed["datastore_settings"],
            ),
            sampling_strategy=seed["sampling_strategy"],
        )
    # add the configured columns here using the supported v1 column types
    return builder
```

**Step 4: Run test to verify it passes**

Run: `python -m unittest tests.test_nvidia_data_designer.BuilderMappingTests.test_build_request_spec_includes_seed_dataset_when_present -v`

Expected: PASS

**Step 5: Commit**

```bash
git add tools/nvidia_data_designer.py tests/test_nvidia_data_designer.py
git commit -m "feat: map fixtures into data designer builders"
```

### Task 5: Add CLI run behavior and artifact writing

**Files:**
- Modify: `/Users/arjun/Documents/transcription-pipeline-skill/tools/nvidia_data_designer.py`
- Test: `/Users/arjun/Documents/transcription-pipeline-skill/tests/test_nvidia_data_designer.py`

**Step 1: Write the failing test**

```python
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools import nvidia_data_designer


class RunScenarioTests(unittest.TestCase):
    def test_run_scenario_writes_artifact_summary(self):
        class FakePreview:
            dataset = [{"rating": 5, "review": "Solid product"}]

            def display_sample_record(self):
                return self.dataset[0]

        class FakeClient:
            def preview(self, builder, num_records):
                return FakePreview()

        with tempfile.TemporaryDirectory() as tmpdir:
            output_dir = Path(tmpdir)
            with patch.object(nvidia_data_designer, "build_client", return_value=FakeClient()):
                artifact = nvidia_data_designer.run_scenario(
                    "retail_reviews_basic",
                    api_key="test-key",
                    output_dir=output_dir,
                )
            payload = json.loads(artifact.read_text(encoding="utf-8"))
            self.assertEqual(payload["scenario"], "retail_reviews_basic")
            self.assertEqual(payload["rows"], 1)
```

**Step 2: Run test to verify it fails**

Run: `python -m unittest tests.test_nvidia_data_designer.RunScenarioTests.test_run_scenario_writes_artifact_summary -v`

Expected: FAIL because `run_scenario` does not exist yet.

**Step 3: Write minimal implementation**

```python
def build_client(api_key: str):
    from nemo_microservices.data_designer.essentials import NeMoDataDesignerClient

    return NeMoDataDesignerClient(
        base_url="https://ai.api.nvidia.com/v1/nemo/dd",
        default_headers={"Authorization": f"Bearer {api_key}"},
    )


def run_scenario(name: str, api_key: str, output_dir: Path | None = None) -> Path:
    scenario = load_scenario(name)
    client = build_client(api_key)
    builder = build_config_builder(scenario)
    builder.validate()
    preview = client.preview(builder, num_records=scenario["preview"]["num_records"])
    destination_dir = output_dir or default_output_dir()
    destination_dir.mkdir(parents=True, exist_ok=True)
    artifact_path = destination_dir / f"{name}-{timestamp()}.json"
    sample = preview.display_sample_record() if hasattr(preview, "display_sample_record") else None
    rows = len(preview.dataset) if hasattr(preview, "dataset") else 0
    artifact_path.write_text(
        json.dumps(
            {
                "scenario": name,
                "rows": rows,
                "sample_record": sample,
            },
            indent=2,
            default=str,
        ),
        encoding="utf-8",
    )
    return artifact_path
```

**Step 4: Run test to verify it passes**

Run: `python -m unittest tests.test_nvidia_data_designer.RunScenarioTests.test_run_scenario_writes_artifact_summary -v`

Expected: PASS

**Step 5: Commit**

```bash
git add tools/nvidia_data_designer.py tests/test_nvidia_data_designer.py
git commit -m "feat: add nvidia scenario execution and artifacts"
```

### Task 6: Add the command-line interface

**Files:**
- Modify: `/Users/arjun/Documents/transcription-pipeline-skill/tools/nvidia_data_designer.py`
- Test: `/Users/arjun/Documents/transcription-pipeline-skill/tests/test_nvidia_data_designer.py`

**Step 1: Write the failing test**

```python
import io
import unittest
from contextlib import redirect_stdout
from unittest.mock import patch

from tools import nvidia_data_designer


class CliTests(unittest.TestCase):
    def test_list_scenarios_command_prints_available_names(self):
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            nvidia_data_designer.main(["list-scenarios"])
        output = stdout.getvalue()
        self.assertIn("retail_reviews_basic", output)
        self.assertIn("patient_notes_seeded", output)
```

**Step 2: Run test to verify it fails**

Run: `python -m unittest tests.test_nvidia_data_designer.CliTests.test_list_scenarios_command_prints_available_names -v`

Expected: FAIL because `main` does not exist yet.

**Step 3: Write minimal implementation**

```python
def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    subparsers = parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser("list-scenarios")

    save_key_parser = subparsers.add_parser("save-key")
    save_key_parser.add_argument("--key", required=True)

    run_parser = subparsers.add_parser("run")
    run_parser.add_argument("scenario")

    args = parser.parse_args(argv)

    if args.command == "list-scenarios":
        for name in list_scenarios():
            print(name)
        return 0

    if args.command == "save-key":
        path = save_key(args.key)
        print(f"Saved key to {path}")
        return 0

    if args.command == "run":
        artifact = run_scenario(args.scenario, api_key=load_key())
        print(f"Scenario {args.scenario} complete")
        print(f"Artifact: {artifact}")
        return 0

    parser.error("Unknown command")
```

**Step 4: Run test to verify it passes**

Run: `python -m unittest tests.test_nvidia_data_designer.CliTests.test_list_scenarios_command_prints_available_names -v`

Expected: PASS

**Step 5: Commit**

```bash
git add tools/nvidia_data_designer.py tests/test_nvidia_data_designer.py
git commit -m "feat: add nvidia harness cli"
```

### Task 7: Run verification and live scenarios

**Files:**
- Modify if needed: `/Users/arjun/Documents/transcription-pipeline-skill/README.md`
- Verify output: `/Users/arjun/Documents/transcription-pipeline-skill/output/nvidia-data-designer/`

**Step 1: Write the failing test**

There is no new unit test in this task. This task is verification-focused. Apply @verification-before-completion and keep code unchanged unless verification exposes a defect.

**Step 2: Run test to verify it fails**

Not applicable. The verification gate here is manual integration execution.

**Step 3: Write minimal implementation**

```text
- add a short README section documenting:
  - dependency installation
  - `save-key`
  - `list-scenarios`
  - `run retail_reviews_basic`
  - `run patient_notes_seeded`
```

**Step 4: Run test to verify it passes**

Run:

```bash
python -m unittest tests.test_nvidia_data_designer -v
python -m pip install -r requirements-nvidia-data-designer.txt
python tools/nvidia_data_designer.py list-scenarios
python tools/nvidia_data_designer.py save-key --key '<redacted>'
python tools/nvidia_data_designer.py run retail_reviews_basic
python tools/nvidia_data_designer.py run patient_notes_seeded
```

Expected:

- unit tests PASS
- scenario listing prints both fixture names
- key save succeeds without printing the full key
- both live preview commands return success summaries
- timestamped artifact files appear under `output/nvidia-data-designer/`

**Step 5: Commit**

```bash
git add README.md tools/nvidia_data_designer.py tests/test_nvidia_data_designer.py fixtures/nvidia-data-designer requirements-nvidia-data-designer.txt .gitignore
git commit -m "feat: add nvidia data designer preview harness"
```

## Notes For Execution

- Use @test-driven-development for every code task above.
- Treat the live NVIDIA requests as manual integration verification, not as default automated tests.
- Keep the supported fixture schema narrow until a real need appears for more column types.
- Do not print credential material during execution.
- This workspace is not currently a Git repository, so commit steps are blocked unless the directory is later attached to Git.
