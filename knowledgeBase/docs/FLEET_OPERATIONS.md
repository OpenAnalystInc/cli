# Fleet Operations Guide

This document covers the GPU fleet architecture, scheduler, monitoring, and
troubleshooting for the transcription pipeline.

## Fleet Architecture

### GPU Hosts

| Host | GPU | VRAM | transcribe_workers | io_workers | File Kinds |
|------|-----|------|--------------------|------------|------------|
| a100-west-1 | A100 | 40GB | 6 | 12 | media |
| a100-east-1 | A100 | 40GB | 6 | 12 | media |
| a100-west-2 | A100 | 40GB | 6 | 12 | media |
| gh200-east-1 | GH200 | 96GB HBM3 | 10 | 20 | document,image,media |
| gh200-east-2 | GH200 | 96GB HBM3 | 10 | 20 | document,image,media |
| v100-east-1 | V100 x8 | 16GB each | 1 (per GPU) | 8 | media |
| v100-east-2 | V100 x8 | 16GB each | 1 (per GPU) | 8 | media |

Configuration: `profiles/workers.json`

### VRAM Budget Rules

Each whisper large-v3 instance uses ~5-6GB VRAM. Marker OCR models use ~10-15GB.

- **A100 (40GB)**: tw=6 uses ~30GB, leaving ~10GB headroom
- **GH200 (96GB)**: tw=10 uses ~60GB, leaving ~28GB for marker OCR models
- **V100 (16GB)**: tw=1 per GPU, ~5GB used, plenty of headroom

If VRAM is exhausted (nvidia-smi shows >95% memory used), reduce
`transcribe_workers` in `profiles/workers.json`.

### Processing Models

- **media** (audio/video): FasterWhisper (ctranslate2), GPU-bound
- **document** (PDF, XLSX, DOCX): marker-pdf OCR, GPU+CPU
- **image** (PNG, JPG): marker OCR, GPU+CPU
- **native documents** (HTML, HTM, CSV, TXT, MD): CPU-only text extraction

### GH200 Characteristics

GH200s show low GPU utilization (~12-15%) even when fully loaded. This is
normal: whisper inference is memory-bandwidth bound, not compute bound, on the
massive HBM3 bandwidth. CPU utilization and VRAM usage are better indicators of
activity. The combined `document,image,media` file kinds help fill the remaining
GPU capacity with marker OCR alongside whisper.

## Scheduler

### Components

- **Scheduler**: `tools/multi_worker_scheduler.py`
- **Cron wrapper**: `tools/cron_scheduler.sh`
- **Batch launcher**: `tools/run_catalog_batch.sh`
- **Remote launcher**: `tools/run_saved_profile.sh`
- **Heartbeat monitor**: `tools/worker_heartbeat.py`

### Cron Schedule

```
*/15 * * * * bash /path/to/tools/cron_scheduler.sh
```

The cron runs in `fresh-only` mode: only assigns idle workers, never restarts
or reassigns active workers.

### Host Control Modes

- **fresh-only** (default for cron): Only assigns idle workers. Active workers
  are left alone regardless of whether they are "owned" by a scheduler batch or
  unmanaged. Prevents process pile-up.
- **restart-owned**: Can reassign owned workers to different folders. Use only
  for manual rebalancing, not for cron. Risks pile-up if the remote kill fails
  during the startup window.

### Launch Placeholder Guard

When the scheduler decides to launch a worker, it inserts a placeholder
`job_runs` record in the DB before starting `run_saved_profile.sh`. This
prevents the next scheduler cycle from seeing the worker as idle during the
60-120s startup window (SSH + git pull + venv setup + model download).

- Placeholder TTL: 15 minutes (matches the cron interval)
- Any `running` job_run with no matching remote process is marked as
  `failed/stale_process` and the worker becomes eligible again
- This applies to both scheduler-owned placeholders (`sched-*` run_ids)
  and worker-created records whose process exited without updating the DB

### Host Reservations

`profiles/host_reservations.json` maps workers to preferred folders. Reserved
folders bypass the `--pending-transcription-only` filter — even folders with
`pending_file_count=0` (files not yet inventoried) are assignable to their
reserved worker. The worker discovers files via `mega-ls` at runtime.

### Scheduling Policies

- **max-throughput**: Assigns workers to the largest pending folders first
- **equal-courses**: Distributes workers across distinct folders before splitting
- **equal-pending-work**: Balances by pending file count

### File Kinds and Lane Exclusion

Workers with combined file kinds (e.g., `document,image,media`) require
exclusive folder access. The scheduler sorts all-kinds-only workers first to
avoid lane exclusion starvation.

Canonical ordering is alphabetical: `document,image,media` (not
`media,image,document`). The `canonicalize_file_kinds()` function handles this.

### Prefetch Strategy

All hosts use a prefetch process to decouple MEGA downloads from GPU
transcription. One `mega_prefetch.py` process per physical host downloads
assigned folders (plus lookahead folders) to `{runtime_root}/prefetch/`.
GPU workers read from the prefetch directory via `--prefetch-root` instead
of calling `mega-get` directly. This keeps GPUs at high utilization
(observed 95% on V100s vs 40% without prefetch on IO-bound A100s).

For multi-GPU hosts (V100s), the prefetch also serves as a shared download
layer — one download serves all 8 GPUs on the same NFS volume.

### Heartbeat Monitor

`tools/worker_heartbeat.py` runs as a background daemon launched by
`cron_scheduler.sh`. It polls every 60s to detect stuck or dead workers by
checking log file mtimes and local launcher PIDs.

**Health classification:**

- **healthy**: Log updated within threshold
- **stale**: Log stale but local launcher PID is alive — warn only
- **dead**: Log stale AND launcher PID gone — mark `failed/heartbeat_timeout`
  in the DB and optionally trigger the scheduler to re-dispatch

**Thresholds:**

- Active workers: 300s (5 min) stale threshold
- Bootstrapping workers (no `PHASE runner_start` in log): 600s (10 min) grace

**PID sidecar files:** The scheduler writes a `.pid` file alongside each
worker/prefetch log (e.g., `WORKER_NAME.pid`). The heartbeat monitor reads
these to determine if the local launcher process is still alive.

**Worker heartbeats:** Remote workers emit `HEARTBEAT <timestamp>` lines to
their log via the incremental sync loop, keeping the log mtime fresh as long
as the worker is alive.

**Cron integration:** `cron_scheduler.sh` launches the heartbeat monitor if
not already running (tracked via `/tmp/worker_heartbeat.pid`). Only one
instance runs at a time.

### Manual Scheduler Run

```bash
python -m tools.multi_worker_scheduler \
  --profile profiles/arjun-lambda-mega.env.local \
  --workers-file profiles/workers.json \
  --scheduling-policy max-throughput \
  --host-control-mode fresh-only \
  --pending-transcription-only \
  --top-level-only
```

Or trigger the cron wrapper directly:

```bash
bash tools/cron_scheduler.sh
```

Check the output in:
`transcriptions/_scheduler/cron/YYYYMMDD.log`

## Monitoring

### SwiftBar Menu Bar Plugin

Location: `~/Library/SwiftBarPlugins/gpu-monitor.30s.sh`

Refreshes every 30 seconds. Shows per-host GPU utilization, VRAM, CPU usage,
and the current folder being processed. Icon colors:

- Green (flame): GPU or CPU utilization > 50%
- Orange (arrow): utilization > 0%
- Gray (zzz): idle

### Quick SSH Diagnostics

```bash
# GPU status
ssh -i $KEY ubuntu@$HOST 'nvidia-smi --query-gpu=memory.used,memory.total,utilization.gpu --format=csv,noheader'

# Process count and details
ssh -i $KEY ubuntu@$HOST 'ps aux | grep transcribe_mega | grep -v grep'

# Full monitoring
bash tools/monitor_lambda_host.sh --host $HOST --user ubuntu --ssh-key $KEY --mode snapshot
```

### Catalog Queries

```bash
# Pending files by kind (active folders only)
python3 -c "
from tools import catalog_jobs
from pathlib import Path
db = Path('catalog/transcription_catalog.db')
with catalog_jobs.connect_db(db) as conn:
    for r in conn.execute('''
        SELECT kind, transcript_status, COUNT(*) FROM files
        WHERE COALESCE((SELECT excluded FROM folders WHERE folders.path = files.parent_path), 0) = 0
        GROUP BY kind, transcript_status ORDER BY kind, transcript_status
    ''').fetchall():
        print(f'{r[0] or \"unknown\":12} {r[1] or \"unknown\":20} {r[2]:>8}')
"
```

## Troubleshooting

### Process Pile-Up

**Symptom**: Multiple `transcribe_mega_folder.py` processes on one host, VRAM
near 100%.

**Cause**: Old `restart-owned` cron mode, or manual scheduler overlapping with
cron during the startup window. On multi-GPU hosts (V100s), simultaneous
bootstrap of all 8 GPU workers can cause `apt-get` lock contention — 7 of 8
may die, then the next scheduler cycle relaunches them, stacking up.

**Fix**:
1. Kill all extra processes — keep only the newest PID per GPU slot:
   `ssh -i $KEY ubuntu@$HOST 'ps aux | grep transcribe_mega | grep -v grep'`
2. Kill local `run_saved_profile.sh` launchers:
   `ps aux | grep run_saved_profile | grep HOST_IP | awk '{print $2}' | xargs kill`
3. Ensure cron uses `--host-control-mode fresh-only`
4. The placeholder DB guard and automatic `failed/stale_process` cleanup prevent
   future pile-up — stale `running` job_runs with no matching remote process are
   auto-marked as failed after 15 minutes

### Worker Not Starting

**Symptom**: Scheduler shows "launch" but no process appears on the remote host.

**Check**:
1. Scheduler log: `transcriptions/_scheduler/cron/YYYYMMDD.log`
2. Worker-specific log: `transcriptions/_scheduler/sched-YYYYMMDD-HHMMSS/WORKER_NAME.log`
3. SSH connectivity: `ssh -i $KEY ubuntu@$HOST 'echo OK'`
4. Disk space: `ssh -i $KEY ubuntu@$HOST 'df -h /lambda/nfs/mega'`
5. MEGA session: `ssh -i $KEY ubuntu@$HOST 'mega-whoami'`

### VRAM Exhaustion / OOM

**Symptom**: nvidia-smi shows >95% VRAM, 0% GPU utilization, process may crash.

**Fix**:
1. Kill all processes on the host
2. Reduce `transcribe_workers` in `profiles/workers.json`
3. Commit, push, and let cron relaunch

### GH200 Low GPU Utilization

**Not a bug**. GH200 GPU utilization is typically 12-15% for whisper because
inference is memory-bandwidth bound. Check CPU utilization and VRAM usage
instead. The combined media+OCR workload helps fill idle cycles.

### Excluded Folders Inflating Pending Counts

Pending file counts include excluded folders. To see the real workload:

```sql
SELECT COUNT(*) FROM files f
JOIN folders fo ON f.parent_path = fo.path
WHERE f.transcript_status = 'pending' AND COALESCE(fo.excluded, 0) = 0
```

### Stale Job Run Blocking Worker

If a worker appears "active" in the scheduler but has no remote process, a stale
`running` job_run may exist. The scheduler automatically detects and marks these
as `failed/stale_process` after 15 minutes — this applies to all run_ids, not
just scheduler-owned (`sched-*`) placeholders. To force cleanup manually:

```sql
UPDATE job_runs SET status = 'failed', error_text = 'manual_cleanup'
WHERE status = 'running' AND worker_name = 'WORKER_NAME';
```

### Reserved Folders Showing Zero Pending

Reserved folders often have `pending_file_count=0` because files haven't been
inventoried yet (the catalog knows the folder exists from `sync_account` but
hasn't enumerated its contents). This is normal — reserved folders bypass the
`--pending-transcription-only` filter and the worker discovers files via
`mega-ls` at runtime.

### Heartbeat Monitor Not Detecting Dead Workers

**Check**:
1. Verify the monitor is running: `cat /tmp/worker_heartbeat.pid && ps -p $(cat /tmp/worker_heartbeat.pid)`
2. Check `.pid` sidecar files exist alongside worker logs in `transcriptions/_scheduler/sched-*/`
3. Verify workers emit `HEARTBEAT` lines: `tail -5 transcriptions/_scheduler/sched-*/WORKER_NAME.log`
4. If the monitor isn't running, trigger `bash tools/cron_scheduler.sh` — it auto-launches the monitor
