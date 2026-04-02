# Video Analysis Pipeline — Usage Guide

This guide covers a separate video-analysis workflow. It is not the primary
operator runbook for the MEGA transcription system. For the current
transcription process, use [../README.md](../README.md),
[TRANSCRIPTION_OBJECTIVES.md](TRANSCRIPTION_OBJECTIVES.md),
[TRANSCRIPTION_PROCESS.md](TRANSCRIPTION_PROCESS.md), and
[RUNBOOKS.md](RUNBOOKS.md).

Analyze existing video courses and ad libraries to extract creative structure, then recreate ads using the LTX generation pipeline.

```
MEGA S4 Video Library
        |
        v
  Pass 1: Structure Extraction (MiniCPM-o)
  Scene segmentation, OCR, timestamps, audio summary
        |
        v
  Pass 2: Grounded QA (Molmo2-8B)
  Spatial evidence for value props, logos, compliance
        |
        v
  Pass 3: Creative Brief (MiniCPM-o)
  Hook breakdown, scripts, variant ideas
        |
        v
  generate-ltx-specs
        |
        v
  LTX Orchestrator --> New Video Variants
```

---

## 1. Prerequisites & Setup

### Required Credentials

| Credential | Source | Purpose |
|---|---|---|
| `LAMBDA_API_KEY` | [Lambda Labs dashboard](https://cloud.lambda.ai) | GPU instance provisioning |
| `SSH_KEY_NAME` + `SSH_KEY_PATH` | Lambda Labs SSH keys page | SSH into GPU instance |
| `HF_TOKEN` | [Hugging Face tokens](https://huggingface.co/settings/tokens) | Download gated models |
| `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` | MEGA S4 dashboard | Access video files in MEGA S4 |

### How Credentials Are Loaded

The pipeline checks two places, in order:

1. **Environment variables** (highest priority)
2. **`assets/` folder** files (fallback)

```bash
# Option A: Export environment variables
export LAMBDA_API_KEY="your_key"
export SSH_KEY_NAME="your_registered_key_name"
export SSH_KEY_PATH="~/.ssh/your_key.pem"
export HF_TOKEN="hf_..."
export AWS_ACCESS_KEY_ID="your_mega_key"
export AWS_SECRET_ACCESS_KEY="your_mega_secret"
```

```bash
# Option B: Place credentials in assets/ folder
echo "your_lambda_key" > assets/lambda_api_key.txt
echo "your_ssh_key_name" > assets/ssh_key_name.txt
echo "~/.ssh/your_key.pem" > assets/lambda_ssh_key_path.txt
```

For MEGA S4 specifically, the pipeline reads `assets/mega_s4_creds` in this format:

```
AWS_ACCESS_KEY_ID="your_mega_access_key"
AWS_SECRET_ACCESS_KEY="your_mega_secret_key"
```

### Install Dependencies

```bash
pip install pyyaml        # Required for YAML spec files
pip install awscli         # Required for MEGA S4 access
chmod +x tools/analyze     # Make CLI executable
```

### Verify Connectivity

```bash
# Lambda Labs API
bash tools/lambda_api_manager.sh list-instance-types

# MEGA S4
aws s3 ls s3://your-bucket/ --endpoint-url=https://s3.eu-central-1.s4.mega.io
```

---

## 2. Working with MEGA S4

MEGA S4 is 100% AWS S3 API-compatible. Every command uses the standard `aws s3` CLI with one addition: `--endpoint-url=https://s3.eu-central-1.s4.mega.io`.

### Browsing Your Video Library

```bash
# List all buckets (if your credentials have ListAllMyBuckets permission)
aws s3 ls --endpoint-url=https://s3.eu-central-1.s4.mega.io

# List top-level contents of a bucket
aws s3 ls s3://ad-library/ --endpoint-url=https://s3.eu-central-1.s4.mega.io

# List everything under a prefix (recursive)
aws s3 ls s3://ad-library/facebook-ads/2025-Q1/ --recursive \
  --endpoint-url=https://s3.eu-central-1.s4.mega.io
```

Example output:

```
2025-01-15 10:30:00    1048576 facebook-ads/2025-Q1/competitor_hook_reel.mp4
2025-01-15 10:31:00    2097152 facebook-ads/2025-Q1/product_demo_30s.mp4
2025-01-15 10:32:00    5242880 facebook-ads/2025-Q1/ugc_testimonial_60s.mp4
```

### Downloading Videos

```bash
# Download a single video
aws s3 cp s3://ad-library/facebook-ads/2025-Q1/competitor_hook_reel.mp4 \
  ./downloads/ \
  --endpoint-url=https://s3.eu-central-1.s4.mega.io

# Download all videos under a prefix
aws s3 sync s3://ad-library/facebook-ads/2025-Q1/ \
  ./downloads/facebook-ads-q1/ \
  --endpoint-url=https://s3.eu-central-1.s4.mega.io

# Download only .mp4 files (exclude everything else)
aws s3 sync s3://ad-library/facebook-ads/2025-Q1/ \
  ./downloads/facebook-ads-q1/ \
  --exclude "*" --include "*.mp4" \
  --endpoint-url=https://s3.eu-central-1.s4.mega.io
```

### Uploading Videos to MEGA S4

```bash
# Upload a single file
aws s3 cp ./my_ad.mp4 s3://ad-library/facebook-ads/2025-Q1/my_ad.mp4 \
  --endpoint-url=https://s3.eu-central-1.s4.mega.io

# Upload an entire folder
aws s3 sync ./local-ad-folder/ s3://ad-library/facebook-ads/2025-Q1/ \
  --endpoint-url=https://s3.eu-central-1.s4.mega.io

# Upload course videos to a dedicated prefix
aws s3 sync ./course-recordings/ s3://ad-library/courses/positioning/ \
  --endpoint-url=https://s3.eu-central-1.s4.mega.io
```

### How the Pipeline Uses MEGA S4 Internally

When you set `input.source: mega_s4` in your analysis spec, the pipeline:

1. Provisions a Lambda GPU instance
2. Configures AWS credentials on the instance (from your env vars or `assets/mega_s4_creds`)
3. Runs `aws s3 cp` on the GPU instance to download each video directly
4. Processes the video through all 3 passes
5. Downloads results back to your local machine via SCP

You never need to manually download videos first. The pipeline handles the transfer from MEGA S4 to GPU instance automatically.

---

## 3. Writing Analysis Specs

Analysis specs are YAML files that tell the pipeline what to analyze and how. They live in the `plans/` directory.

### Full Schema Reference

```yaml
# --- Required fields ---
job_id: string              # Alphanumeric, dots, underscores, hyphens only
job_type: analysis           # Must be "analysis"
backend: ssh_oss             # Must be "ssh_oss"

# --- Input block (required) ---
input:
  source: mega_s4 | local   # Where videos live

  # For mega_s4:
  bucket: string             # MEGA S4 bucket name (required)
  prefix: string             # S3 key prefix to scan (optional, alternative to paths)
  paths:                     # Specific file keys within the bucket (optional)
    - path/to/video.mp4
  endpoint_url: string       # Default: https://s3.eu-central-1.s4.mega.io

  # For local:
  paths:                     # Absolute paths on your machine (required)
    - /Users/you/Downloads/ad.mp4

# --- Analysis block (required) ---
analysis:
  passes: [1, 2, 3]         # Which passes to run. Default: [1, 2, 3]

  pass_1:                    # Structure Extraction (MiniCPM-o)
    sample_fps: 1            # Frames per second to extract. Range: 0-30. Default: 1
    extract:                 # What to extract. Default: all four
      - scenes
      - timestamps
      - ocr
      - audio_summary
    chunk_minutes: 10        # For long videos: split into N-minute chunks. Default: none

  pass_2:                    # Grounded QA (Molmo2-8B)
    segment_fps: 6           # FPS for key segment re-extraction. Range: 0-30. Default: 6
    questions:               # Custom questions. Default: 4 marketing-focused questions
      - "What exactly is the value prop claim? Quote it."
      - "Is there any compliance risk claim? Where?"
      - "What product attributes are shown visually vs only spoken?"
      - "Where does the brand/logo appear?"

  pass_3:                    # Creative Brief Generation (MiniCPM-o)
    output_format: creative_brief  # creative_brief | json | ltx_specs. Default: creative_brief
    variant_count: 5         # Number of script variants to generate. Range: 1-20. Default: 5

# --- Instance block (optional) ---
instance:
  type: gpu_1x_a100          # Lambda GPU type. Default: gpu_1x_a100
  region: us-east-3          # Lambda region. Default: us-east-3
  auto_terminate: true       # Terminate GPU after completion. Default: false
  ssh_user: ubuntu           # SSH username. Default: ubuntu
  filesystem_names: []       # Persistent filesystems to attach
```

### Example: Minimal Spec (Local Video, Pass 1 Only)

```yaml
job_id: quick_scan
job_type: analysis
backend: ssh_oss

input:
  source: local
  paths:
    - /Users/arjun/Downloads/competitor_ad.mp4

analysis:
  passes: [1]
  pass_1:
    sample_fps: 0.5

instance:
  type: gpu_1x_a100
  auto_terminate: true
```

### Example: MEGA S4 Ad Library (All Passes)

```yaml
job_id: ad_library_q1_2025
job_type: analysis
backend: ssh_oss

input:
  source: mega_s4
  bucket: ad-library
  prefix: facebook-ads/2025-Q1/
  paths:
    - facebook-ads/2025-Q1/competitor_hook_reel.mp4
    - facebook-ads/2025-Q1/product_demo_30s.mp4
  endpoint_url: https://s3.eu-central-1.s4.mega.io

analysis:
  passes: [1, 2, 3]
  pass_1:
    sample_fps: 1
    extract: [scenes, timestamps, ocr, audio_summary]
  pass_2:
    segment_fps: 6
    questions:
      - "What exactly is the value prop claim? Quote it."
      - "Is there any compliance risk claim? Where?"
      - "What product attributes are shown visually vs only spoken?"
      - "Where does the brand/logo appear?"
      - "What emotional trigger is used in the hook?"
  pass_3:
    output_format: creative_brief
    variant_count: 5

instance:
  type: gpu_1x_a100
  region: us-east-3
  auto_terminate: true
```

### Example: Prefix-Only Batch (No Explicit Paths)

When you provide only a `prefix` and no `paths`, the pipeline will list all video files (`*.mp4`, `*.mov`, `*.avi`, etc.) under that prefix and process them all:

```yaml
job_id: full_ad_scan
job_type: analysis
backend: ssh_oss

input:
  source: mega_s4
  bucket: ad-library
  prefix: tiktok-ads/

analysis:
  passes: [1, 2, 3]

instance:
  auto_terminate: true
```

### Example: Long Course Video (With Chunking)

For 30-60+ minute course videos, enable chunking to keep each VLM call within context limits:

```yaml
job_id: course_module3
job_type: analysis
backend: ssh_oss

input:
  source: mega_s4
  bucket: ad-library
  paths:
    - courses/module3_positioning_masterclass.mp4

analysis:
  passes: [1, 2, 3]
  pass_1:
    sample_fps: 0.5
    chunk_minutes: 10
  pass_2:
    segment_fps: 4
    questions:
      - "What key insight or framework is being taught?"
      - "What example or case study is shown?"
      - "What visual aid or slide is displayed?"
      - "What call to action or next step is mentioned?"
  pass_3:
    variant_count: 3

instance:
  type: gpu_1x_a100
  auto_terminate: true
```

How chunking works:
- A 45-minute video with `chunk_minutes: 10` is split into 5 chunks (4x 10min + 1x 5min)
- Pass 1 runs independently on each chunk
- Timestamps are adjusted when merging (chunk 2's scenes start at 600s, not 0s)
- Pass 2 and 3 receive the merged, full-timeline results

---

## 4. Running the Pipeline

### Validate a Spec (No GPU, No Cost)

```bash
./tools/analyze validate plans/sample_analysis.yaml
```

Output:

```json
{
  "has_prefix": true,
  "input_source": "mega_s4",
  "instance_type": "gpu_1x_a100",
  "job_id": "ad_library_q1_2025",
  "job_type": "analysis",
  "passes": [1, 2, 3],
  "valid": true,
  "video_count": 2
}
```

### Run Full Analysis (All 3 Passes)

```bash
./tools/analyze run plans/sample_analysis.yaml
```

This will:
1. Load credentials (env vars or `assets/`)
2. Launch a Lambda GPU instance (A100 by default)
3. Install MiniCPM-o, Molmo2, ffmpeg on the instance
4. Download videos from MEGA S4 to the instance
5. Run Pass 1 (structure extraction) on each video
6. Run Pass 2 (grounded QA) on key segments
7. Run Pass 3 (creative brief) with variant scripts
8. Download all results locally to `runs/{job_id}/`
9. Terminate the instance (if `auto_terminate: true`)

### Run a Single Pass

```bash
# Run only Pass 1 (cheapest, fastest — good for initial scanning)
./tools/analyze run plans/sample_analysis.yaml --pass 1

# Run only Pass 2 (requires Pass 1 results to exist)
./tools/analyze run plans/sample_analysis.yaml --pass 2

# Run only Pass 3 (requires Pass 1 + Pass 2 results to exist)
./tools/analyze run plans/sample_analysis.yaml --pass 3
```

### Generate LTX Specs (Bridge to Video Generation)

After analysis completes, convert the creative brief variants into runnable LTX specs:

```bash
# Generate specs for ALL variants
./tools/analyze generate-ltx-specs --job-id ad_library_q1_2025

# Generate spec for a specific variant (1-indexed)
./tools/analyze generate-ltx-specs --job-id ad_library_q1_2025 --variant 1
./tools/analyze generate-ltx-specs --job-id ad_library_q1_2025 --variant 1 --variant 3
```

Output files are created in `plans/`:

```
plans/ad_library_q1_2025_variant_1.yaml
plans/ad_library_q1_2025_variant_2.yaml
...
```

### Run LTX Generation on the Specs

```bash
# Validate the generated spec first
./ltx-orchestrator plan validate plans/ad_library_q1_2025_variant_1.yaml

# Generate the video
./ltx-orchestrator run plans/ad_library_q1_2025_variant_1.yaml
```

### Terminate the GPU Instance

If `auto_terminate` is `false` (default), terminate manually when done:

```bash
./tools/analyze cleanup ad_library_q1_2025
```

---

## 5. Understanding Output

### Directory Structure

```
runs/ad_library_q1_2025/
  manifest.json                           # Job metadata, instance info, status
  logs/                                    # Execution logs
  results/
    competitor_hook_reel/                   # Per-video results
      pass_1.json                          # Structure extraction
      pass_2.json                          # Grounded QA
      pass_3.json                          # Creative brief + variants
    product_demo_30s/
      pass_1.json
      pass_2.json
      pass_3.json
```

### Pass 1 Output: Structure Extraction

`runs/{job_id}/results/{video}/pass_1.json`

```json
{
  "video_path": "competitor_hook_reel.mp4",
  "source": "s3://ad-library/facebook-ads/2025-Q1/competitor_hook_reel.mp4",
  "duration_s": 32.5,
  "structure_type": "ugc",
  "scenes": [
    {
      "scene_id": 1,
      "start_s": 0.0,
      "end_s": 2.1,
      "creative_role": "hook",
      "description": "Creator speaks directly to camera, surprised expression",
      "ocr_text": ["I saved $2,000/month"],
      "visual_style": {
        "camera": "handheld selfie",
        "lighting": "natural window",
        "palette": "warm"
      },
      "audio_summary": "Excited voice saying an attention-grabbing stat"
    },
    {
      "scene_id": 2,
      "start_s": 2.1,
      "end_s": 12.0,
      "creative_role": "problem",
      "description": "Split screen showing messy spreadsheets vs clean dashboard",
      "ocr_text": ["Before", "After"],
      "visual_style": {
        "camera": "static split",
        "lighting": "studio balanced",
        "palette": "contrast warm/cool"
      },
      "audio_summary": "Narrator explains the pain of manual reporting"
    }
  ],
  "on_screen_text_inventory": ["I saved $2,000/month", "Before", "After", "Link in bio"],
  "tone_tags": ["authentic", "energetic", "ugc", "testimonial"],
  "audience_hypothesis": "Small business owners frustrated with expensive tools"
}
```

**Creative role labels** used for scene classification:

`hook` | `problem` | `agitation` | `solution` | `demo` | `testimonial` | `social_proof` | `offer` | `cta` | `transition` | `b_roll` | `intro` | `outro` | `talking_head` | `other`

### Pass 2 Output: Grounded QA

`runs/{job_id}/results/{video}/pass_2.json`

```json
{
  "grounded_answers": [
    {
      "question": "What exactly is the value prop claim? Quote it.",
      "answer": "The on-screen text reads: \"I saved $2,000/month on reporting tools\"",
      "segment": {
        "scene_id": 1,
        "creative_role": "hook",
        "start_s": 0.0,
        "end_s": 2.1
      },
      "evidence": [
        {
          "frame_id": 3,
          "timestamp_s": 0.5,
          "coordinates": [412, 230],
          "type": "grounded"
        }
      ]
    },
    {
      "question": "Where does the brand/logo appear?",
      "answer": "Bottom-right corner from 28s onward, small white watermark",
      "segment": {
        "scene_id": 5,
        "creative_role": "cta",
        "start_s": 28.0,
        "end_s": 32.5
      },
      "evidence": [
        {
          "frame_id": 168,
          "timestamp_s": 28.0,
          "coordinates": [890, 510],
          "type": "grounded"
        }
      ]
    }
  ]
}
```

Pass 2 only analyzes key segments — scenes with creative roles: `hook`, `problem`, `demo`, `offer`, `cta`, `testimonial`, `social_proof`. If none match, it falls back to the first 3 scenes.

### Pass 3 Output: Creative Brief + Variant Scripts

`runs/{job_id}/results/{video}/pass_3.json`

```json
{
  "creative_brief": {
    "hook": {
      "text": "Direct-to-camera confession with surprise stat",
      "duration_s": 2.1
    },
    "problem_framing": "Manual reporting wastes founder time",
    "demo_moments": [
      {"timestamp_s": 8.5, "feature": "one-click dashboard"}
    ],
    "offer": "Free trial, no credit card",
    "cta": {
      "text": "Link in bio",
      "placement": "end_card + spoken"
    },
    "text_inventory": ["I saved $2,000/month", "Link in bio", "TRY FREE"],
    "tone_tags": ["ugc", "authentic", "energetic"],
    "audience": "Small business owners, 25-45, frustrated with manual reporting"
  },
  "variant_scripts": [
    {
      "variant_id": 1,
      "hook_angle": "pain-point",
      "script": "Open on frustrated founder at messy desk. She rubs her forehead, then turns to camera: 'I was losing half my Monday to reporting chaos.' Cut to clean dashboard. 'Then I found this.' Demo of one-click report. Close on CTA.",
      "duration_target_s": 30,
      "ltx_spec_hint": {
        "use_case": "ugc",
        "prompt_seed": "INT. HOME OFFICE - DAYTIME. Founder at desk stares at spreadsheets while warm window light creates soft shadows, then turns to camera and speaks with ambient keyboard sounds underneath.",
        "quality_profile": "prod_default",
        "aspect_ratios": ["9:16"]
      }
    },
    {
      "variant_id": 2,
      "hook_angle": "transformation",
      "script": "Split screen: left shows chaos (papers, tabs), right shows clean dashboard. Voiceover: 'Same Monday. Different tool.' Zoom into the dashboard. CTA with offer.",
      "duration_target_s": 15,
      "ltx_spec_hint": {
        "use_case": "product_ad",
        "prompt_seed": "INT. STUDIO SET - DAYTIME. Split screen comparison...",
        "quality_profile": "draft_fast",
        "aspect_ratios": ["9:16", "1:1"]
      }
    }
  ]
}
```

Each variant's `ltx_spec_hint` contains the seed for an LTX job spec. The `generate-ltx-specs` command converts these into full YAML specs.

---

## 6. Bridge to LTX: From Analysis to New Video

The analysis pipeline outputs creative briefs with `ltx_spec_hint` objects. The bridge command converts these into ready-to-run LTX YAML specs.

### Step 1: Generate LTX Specs

```bash
./tools/analyze generate-ltx-specs --job-id ad_library_q1_2025
```

This reads every `pass_3.json` under `runs/ad_library_q1_2025/results/` and produces:

```
Generated: plans/ad_library_q1_2025_variant_1.yaml
Generated: plans/ad_library_q1_2025_variant_2.yaml
Generated: plans/ad_library_q1_2025_variant_3.yaml
...
```

### Step 2: Review and Customize the Generated Spec

The generated spec looks like:

```yaml
job_id: ad_library_q1_2025_variant_1
backend: ssh_oss
use_case: ugc
prompt: "INT. HOME OFFICE - DAYTIME. Founder at desk stares at spreadsheets while warm window light creates soft shadows, then turns to camera and speaks with ambient keyboard sounds underneath."
outputs:
  aspect_ratios:
    - 9:16
  count: 1
  quality_profile: prod_default
```

You can customize it before running. Common additions:

```yaml
# Add a reference photo for subject consistency
references:
  - path: /Users/arjun/Downloads/1.jpg
    frame_idx: 0
    strength: 0.95

# Tune quality
quality_overrides:
  num_frames: 145
  num_inference_steps: 52
  video_cfg_guidance_scale: 4.0
  enhance_prompt: false

# Prevent LTX from appending use-case angle text to your prompt
append_use_case_angle: false

seeds: [3401]
```

### Step 3: Validate and Run

```bash
# Validate
./ltx-orchestrator plan validate plans/ad_library_q1_2025_variant_1.yaml

# Generate the video
./ltx-orchestrator run plans/ad_library_q1_2025_variant_1.yaml

# Collect output
./ltx-orchestrator collect ad_library_q1_2025_variant_1
```

---

## 7. End-to-End Walkthrough

This walkthrough analyzes a batch of competitor Facebook ads from MEGA S4 and generates new ad variants.

### Step 1: Upload Ads to MEGA S4

```bash
# Upload your competitor ads to a dedicated bucket/prefix
aws s3 sync ./competitor-ads/ s3://ad-library/facebook-ads/2025-Q1/ \
  --endpoint-url=https://s3.eu-central-1.s4.mega.io

# Verify the upload
aws s3 ls s3://ad-library/facebook-ads/2025-Q1/ --recursive \
  --endpoint-url=https://s3.eu-central-1.s4.mega.io
```

### Step 2: Write the Analysis Spec

Create `plans/competitor_analysis.yaml`:

```yaml
job_id: competitor_fb_q1
job_type: analysis
backend: ssh_oss

input:
  source: mega_s4
  bucket: ad-library
  prefix: facebook-ads/2025-Q1/
  endpoint_url: https://s3.eu-central-1.s4.mega.io

analysis:
  passes: [1, 2, 3]
  pass_1:
    sample_fps: 1
    extract: [scenes, timestamps, ocr, audio_summary]
  pass_2:
    segment_fps: 6
    questions:
      - "What exactly is the value prop claim? Quote it."
      - "What emotional trigger is used in the hook?"
      - "Where does the brand/logo appear?"
      - "Is there a discount or limited-time offer?"
  pass_3:
    output_format: creative_brief
    variant_count: 5

instance:
  type: gpu_1x_a100
  region: us-east-3
  auto_terminate: true
```

### Step 3: Validate

```bash
./tools/analyze validate plans/competitor_analysis.yaml
```

### Step 4: Run the Analysis

```bash
./tools/analyze run plans/competitor_analysis.yaml
```

Wait for completion. The pipeline processes each video sequentially on the same GPU instance.

### Step 5: Review Results

```bash
# Check the manifest
cat runs/competitor_fb_q1/manifest.json | python3 -m json.tool

# Read the creative brief for one video
cat runs/competitor_fb_q1/results/competitor_hook_reel/pass_3.json | python3 -m json.tool
```

### Step 6: Generate LTX Specs

```bash
# Generate specs for all variants
./tools/analyze generate-ltx-specs --job-id competitor_fb_q1

# Or just the best variant
./tools/analyze generate-ltx-specs --job-id competitor_fb_q1 --variant 1
```

### Step 7: Customize and Run LTX Generation

```bash
# Add your reference photo and tune the generated spec
# (edit plans/competitor_fb_q1_variant_1.yaml as needed)

# Validate
./ltx-orchestrator plan validate plans/competitor_fb_q1_variant_1.yaml

# Generate the new video
./ltx-orchestrator run plans/competitor_fb_q1_variant_1.yaml
```

---

## 8. Long Video Support

Videos over 10 minutes (e.g., course recordings) are automatically chunked when `chunk_minutes` is set.

### How It Works

1. The video is split into segments using ffmpeg's stream copy (fast, no re-encoding):
   ```
   45-minute video, chunk_minutes: 10
   --> chunk_000 (0:00 - 10:00)
   --> chunk_001 (10:00 - 20:00)
   --> chunk_002 (20:00 - 30:00)
   --> chunk_003 (30:00 - 40:00)
   --> chunk_004 (40:00 - 45:00)
   ```

2. Pass 1 runs on each chunk independently

3. Results are merged with timestamp correction:
   - Chunk 0 scene at 5:30 stays at 5:30
   - Chunk 2 scene at 3:00 becomes 23:00 (offset by 20 min)
   - Scene IDs are renumbered sequentially across all chunks
   - OCR text is deduplicated, tone tags are merged

4. Pass 2 targets specific scene timestamps from the merged result

5. Pass 3 receives the full merged analysis

### Recommended Settings for Course Videos

```yaml
pass_1:
  sample_fps: 0.5          # 1 frame every 2 seconds (saves compute for long content)
  chunk_minutes: 10         # 10-minute chunks
```

---

## 9. Troubleshooting

### Missing Credentials

```
ERROR: Missing LAMBDA_API_KEY. Set env var or add assets/lambda_api_key.txt.
```

Ensure your Lambda API key is set as an environment variable or saved in `assets/lambda_api_key.txt`.

### MEGA S4 Access Denied

```
An error occurred (AccessDenied) when calling the ListObjects operation
```

Check:
- `AWS_ACCESS_KEY_ID` and `AWS_SECRET_ACCESS_KEY` are correct
- You're using `--endpoint-url=https://s3.eu-central-1.s4.mega.io`
- The bucket name and prefix are spelled correctly

### No GPU Availability

```
ERROR: Instance did not become active in time.
```

Lambda GPUs can be fully reserved. Try:
- A different instance type (`gpu_1x_a10` instead of `gpu_1x_a100`)
- A different region (`us-east-1` instead of `us-east-3`)
- Waiting and retrying later

### Model Download Failures

If models fail to download on the GPU instance, check:
- Your `HF_TOKEN` is set and has access to gated models
- Network connectivity on the Lambda instance
- Hugging Face Hub isn't rate-limiting you

### Validation Errors

```bash
# Always validate before running to catch spec errors without GPU cost
./tools/analyze validate plans/your_spec.yaml
```

Common validation errors:
- `sample_fps must be between 0 (exclusive) and 30` — FPS out of range
- `bucket is required when source is mega_s4` — Missing MEGA S4 bucket name
- `passes contains invalid pass number: 4` — Only passes 1, 2, 3 are valid
- `variant_count must be an integer between 1 and 20` — Too many variants requested

---

## 10. Quick Reference

### CLI Commands

| Command | Purpose | Cost |
|---|---|---|
| `./tools/analyze validate <spec>` | Check spec validity | Free |
| `./tools/analyze run <spec>` | Run all passes | GPU time |
| `./tools/analyze run <spec> --pass 1` | Run Pass 1 only | GPU time |
| `./tools/analyze generate-ltx-specs --job-id <id>` | Create LTX specs | Free |
| `./tools/analyze generate-ltx-specs --job-id <id> --variant N` | Create specific LTX spec | Free |
| `./tools/analyze cleanup <job_id>` | Terminate GPU | Free |

### MEGA S4 Commands

| Command | Purpose |
|---|---|
| `aws s3 ls s3://bucket/ --endpoint-url=https://s3.eu-central-1.s4.mega.io` | List files |
| `aws s3 ls s3://bucket/prefix/ --recursive --endpoint-url=...` | List all files under prefix |
| `aws s3 cp s3://bucket/key ./local --endpoint-url=...` | Download file |
| `aws s3 cp ./local s3://bucket/key --endpoint-url=...` | Upload file |
| `aws s3 sync s3://bucket/prefix/ ./local/ --endpoint-url=...` | Sync directory down |
| `aws s3 sync ./local/ s3://bucket/prefix/ --endpoint-url=...` | Sync directory up |

### Supported Video Formats

`.mp4` `.mov` `.avi` `.mkv` `.webm` `.m4v` `.flv` `.wmv`

### Instance Types

| Type | GPU | Recommended For |
|---|---|---|
| `gpu_1x_a10` | NVIDIA A10 (24GB) | Budget analysis, short ads only |
| `gpu_1x_a100` | NVIDIA A100 (80GB) | Both models comfortably |
| `gpu_1x_h100` | NVIDIA H100 (80GB) | Recommended default for LTX-2 rendering |
| `gpu_1x_gh200` | NVIDIA GH200 (96GB) | Longest context, largest batches |
