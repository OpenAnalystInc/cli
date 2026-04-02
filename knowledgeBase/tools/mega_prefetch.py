"""Prefetch files from MEGA to a local NFS volume for multi-GPU hosts.

Downloads files from one or more MEGA source folders into a prefetch directory
with `.ready` markers. GPU workers poll for these markers instead of downloading
from MEGA directly, avoiding mega-cmd-server overload when many processes share
a single daemon.

Usage:
    python mega_prefetch.py \
        --mega-source-paths /path/to/folder1,/path/to/folder2 \
        --prefetch-root /lambda/nfs/mega-texas/transcription/prefetch \
        --max-concurrent-downloads 4 \
        --include-file-kinds media
"""

from __future__ import annotations

import argparse
import json
import logging
import os
import shutil
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path, PurePosixPath

# Import shared utilities from the main transcription script.
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from transcribe_mega_folder import (
    MegaCli,
    classify_source_file_kind,
    filter_processable_source_paths,
    normalize_mega_path,
    normalize_selected_file_kinds,
)

LOG_FORMAT = "%(asctime)s [prefetch] %(levelname)s %(message)s"
logging.basicConfig(format=LOG_FORMAT, level=logging.INFO, stream=sys.stdout)
log = logging.getLogger(__name__)

DOWNLOAD_RETRY_ATTEMPTS = 3
DOWNLOAD_RETRY_BACKOFF_SECONDS = 3


MANIFEST_FILENAME = "_manifest.json"


def ready_marker_path(local_path: Path) -> Path:
    """Return the `.ready` marker path for a prefetched file."""
    return local_path.with_name(local_path.name + ".ready")


def write_folder_manifest(
    prefetch_root: Path,
    source_root: PurePosixPath,
    all_files: list[PurePosixPath],
) -> None:
    """Write a manifest of all MEGA files for a folder so GPU workers can skip mega-ls."""
    source_label = source_root.name
    manifest_path = prefetch_root / source_label / MANIFEST_FILENAME
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest = {
        "source_root": str(source_root),
        "files": [str(f) for f in all_files],
    }
    tmp_path = manifest_path.with_suffix(".tmp")
    tmp_path.write_text(json.dumps(manifest, indent=2))
    tmp_path.rename(manifest_path)
    log.info("Wrote manifest for %s: %d files", source_root, len(all_files))


def prefetch_file(
    mega_client: MegaCli,
    mega_path: PurePosixPath,
    local_path: Path,
) -> bool:
    """Download a single file from MEGA and write a `.ready` marker.

    Returns True on success, False on failure (logged, not raised).
    """
    marker = ready_marker_path(local_path)
    if marker.exists():
        log.debug("Already prefetched: %s", mega_path)
        return True

    local_path.parent.mkdir(parents=True, exist_ok=True)

    for attempt in range(1, DOWNLOAD_RETRY_ATTEMPTS + 1):
        try:
            mega_client.download_file(mega_path, local_path)
            marker.touch()
            log.info("Prefetched: %s -> %s", mega_path, local_path)
            return True
        except Exception as exc:
            log.warning(
                "Download attempt %d/%d failed for %s: %s",
                attempt,
                DOWNLOAD_RETRY_ATTEMPTS,
                mega_path,
                exc,
            )
            if attempt < DOWNLOAD_RETRY_ATTEMPTS:
                time.sleep(DOWNLOAD_RETRY_BACKOFF_SECONDS * attempt)

    log.error("Failed to prefetch after %d attempts: %s", DOWNLOAD_RETRY_ATTEMPTS, mega_path)
    return False


def run_prefetch(
    mega_source_paths: list[str],
    prefetch_root: Path,
    max_concurrent_downloads: int,
    selected_file_kinds: set[str],
) -> dict[str, int]:
    """Prefetch files from multiple MEGA source folders.

    Returns a dict with counts: {"downloaded": N, "skipped": N, "failed": N}.
    """
    mega_client = MegaCli()
    stats = {"downloaded": 0, "skipped": 0, "failed": 0}

    # Collect files to download per folder, then interleave so that
    # downloads round-robin across folders.  This ensures all GPU workers
    # get data quickly instead of one folder monopolising the queue.
    per_folder_tasks: list[list[tuple[PurePosixPath, Path, str]]] = []

    for raw_path in mega_source_paths:
        source_root = normalize_mega_path(raw_path)
        source_label = source_root.name
        log.info("Listing files in %s ...", source_root)

        try:
            all_files = mega_client.list_files(source_root)
        except Exception as exc:
            log.error("Failed to list %s: %s", source_root, exc)
            continue

        processable = filter_processable_source_paths(all_files)
        log.info(
            "Found %d processable files out of %d total in %s",
            len(processable),
            len(all_files),
            source_root,
        )

        # Write manifest so GPU workers can skip mega-ls.
        write_folder_manifest(prefetch_root, source_root, all_files)

        folder_tasks: list[tuple[PurePosixPath, Path, str]] = []
        for mega_path in processable:
            file_kind = classify_source_file_kind(mega_path)
            if file_kind not in selected_file_kinds:
                continue
            relative_path = mega_path.relative_to(source_root)
            local_path = prefetch_root / source_label / relative_path
            marker = ready_marker_path(local_path)
            if marker.exists():
                stats["skipped"] += 1
                continue
            folder_tasks.append((mega_path, local_path, source_label))
        if folder_tasks:
            per_folder_tasks.append(folder_tasks)

    # Interleave: take one task from each folder in round-robin order.
    download_tasks: list[tuple[PurePosixPath, Path, str]] = []
    max_len = max((len(ft) for ft in per_folder_tasks), default=0)
    for idx in range(max_len):
        for folder_tasks in per_folder_tasks:
            if idx < len(folder_tasks):
                download_tasks.append(folder_tasks[idx])

    log.info(
        "Prefetch plan: %d to download, %d already done, %d folders",
        len(download_tasks),
        stats["skipped"],
        len(per_folder_tasks),
    )

    if not download_tasks:
        return stats

    # Download with controlled concurrency.
    with ThreadPoolExecutor(max_workers=max_concurrent_downloads) as pool:
        future_to_path = {
            pool.submit(prefetch_file, mega_client, mega_path, local_path): mega_path
            for mega_path, local_path, _label in download_tasks
        }
        for future in as_completed(future_to_path):
            mega_path = future_to_path[future]
            try:
                success = future.result()
                if success:
                    stats["downloaded"] += 1
                else:
                    stats["failed"] += 1
            except Exception as exc:
                log.error("Unexpected error prefetching %s: %s", mega_path, exc)
                stats["failed"] += 1

    return stats


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Prefetch MEGA files to NFS volume for multi-GPU hosts",
    )
    parser.add_argument(
        "--mega-source-paths",
        required=True,
        help="Comma-separated list of MEGA source folder paths",
    )
    parser.add_argument(
        "--prefetch-root",
        required=True,
        help="Local directory root for prefetched files",
    )
    parser.add_argument(
        "--max-concurrent-downloads",
        type=int,
        default=8,
        help="Maximum number of concurrent MEGA downloads (default: 8)",
    )
    parser.add_argument(
        "--include-file-kinds",
        default=None,
        help="Comma-separated file kinds to prefetch (default: all processable)",
    )
    parser.add_argument(
        "--loop",
        action="store_true",
        help="Run continuously, re-scanning for new files every --loop-interval seconds",
    )
    parser.add_argument(
        "--loop-interval",
        type=int,
        default=120,
        help="Seconds between prefetch scans when --loop is set (default: 120)",
    )

    args = parser.parse_args()

    # Support | as delimiter (commas can appear in folder names).
    raw = args.mega_source_paths
    sep = "|" if "|" in raw else ","
    mega_source_paths = [p.strip() for p in raw.split(sep) if p.strip()]
    if not mega_source_paths:
        parser.error("--mega-source-paths must contain at least one path")

    prefetch_root = Path(args.prefetch_root)
    prefetch_root.mkdir(parents=True, exist_ok=True)

    selected_file_kinds = normalize_selected_file_kinds(args.include_file_kinds)

    log.info(
        "Prefetch starting: sources=%s root=%s concurrency=%d kinds=%s loop=%s",
        mega_source_paths,
        prefetch_root,
        args.max_concurrent_downloads,
        sorted(selected_file_kinds),
        args.loop,
    )

    while True:
        stats = run_prefetch(
            mega_source_paths=mega_source_paths,
            prefetch_root=prefetch_root,
            max_concurrent_downloads=args.max_concurrent_downloads,
            selected_file_kinds=selected_file_kinds,
        )
        log.info(
            "Prefetch round complete: downloaded=%d skipped=%d failed=%d",
            stats["downloaded"],
            stats["skipped"],
            stats["failed"],
        )

        if not args.loop:
            break

        log.info("Sleeping %d seconds before next scan...", args.loop_interval)
        time.sleep(args.loop_interval)


if __name__ == "__main__":
    main()
