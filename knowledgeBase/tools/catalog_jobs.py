from __future__ import annotations

import argparse
import csv
import json
import os
import re
import shlex
import sqlite3
import subprocess
import time
from pathlib import Path, PurePosixPath

try:
    import transcribe_mega_folder
except ModuleNotFoundError:  # pragma: no cover - import path differs between script and package usage.
    from tools import transcribe_mega_folder


FM_URL_RE = re.compile(r"^https://mega\.(?:nz|co\.nz)/fm/([A-Za-z0-9_-]+)")
MEGA_ERR_PREFIX_RE = re.compile(r"^\[err:\s*[^\]]+\]\s*")
DEFAULT_DB_PATH = Path(__file__).resolve().parents[1] / "catalog" / "transcription_catalog.db"
DEFAULT_LOCAL_TRANSCRIPT_DIR = Path(__file__).resolve().parents[1] / "transcriptions"
DEFAULT_KB_DB_PATH = Path(__file__).resolve().parents[1] / "catalog" / "transcript_knowledge_base.db"
DEFAULT_KB_QDRANT_PATH = Path(__file__).resolve().parents[1] / "catalog" / "transcript_qdrant"
URL_ONLY_RE = re.compile(r"^https?://\S+$")


def utc_now() -> str:
    return transcribe_mega_folder.utc_timestamp()


def normalize_db_path(db_path: str | Path | None) -> Path:
    return Path(db_path) if db_path is not None else DEFAULT_DB_PATH


def parse_profile(profile_path: str | None) -> dict[str, str]:
    if not profile_path:
        return {}
    values: dict[str, str] = {}
    for raw_line in Path(profile_path).read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        key, value = line.split("=", 1)
        values[key.strip()] = value.strip().strip('"').strip("'")
    return values


def row_to_dict(row: sqlite3.Row | None) -> dict | None:
    return dict(row) if row is not None else None


def classify_file_kind(path: str | PurePosixPath) -> str:
    return transcribe_mega_folder.classify_source_file_kind(path)


def is_processable_source(path: str | PurePosixPath, processable_paths: set[PurePosixPath]) -> bool:
    normalized = PurePosixPath(path)
    return normalized in processable_paths


def expected_transcript_path(
    path: str | PurePosixPath,
    all_paths: set[PurePosixPath],
    processable_paths: set[PurePosixPath],
) -> PurePosixPath | None:
    candidate = PurePosixPath(path)
    if not is_processable_source(candidate, processable_paths):
        return None
    file_kind = classify_file_kind(candidate)
    return transcribe_mega_folder.build_generated_output_path(
        candidate,
        candidate.parent,
        source_file_kind=file_kind,
        output_root=candidate.parent,
    )


def expected_companion_prefix(
    path: str | PurePosixPath,
    processable_paths: set[PurePosixPath],
) -> tuple[PurePosixPath, str] | None:
    candidate = PurePosixPath(path)
    if not is_processable_source(candidate, processable_paths):
        return None
    file_kind = classify_file_kind(candidate)
    marker = ".timestamps." if file_kind == "media" else ".structured."
    return candidate.parent, f"{candidate.stem}{marker}"


def find_latest_companion_output(
    path: str | PurePosixPath,
    all_paths: set[PurePosixPath],
    processable_paths: set[PurePosixPath],
) -> PurePosixPath | None:
    prefix_info = expected_companion_prefix(path, processable_paths)
    if prefix_info is None:
        return None
    parent_path, prefix = prefix_info
    matches = sorted(
        candidate
        for candidate in all_paths
        if candidate.parent == parent_path and candidate.name.startswith(prefix) and candidate.suffix.lower() == ".txt"
    )
    return matches[-1] if matches else None


def has_paired_outputs(
    transcript_path: str | None,
    companion_output_path: str | None,
) -> bool:
    return bool(transcript_path and companion_output_path)


def candidate_processable_paths(all_paths: set[PurePosixPath]) -> set[PurePosixPath]:
    return set(transcribe_mega_folder.filter_processable_source_paths(all_paths))


def build_transcript_preview(text: str, limit: int = 120) -> str:
    normalized = " ".join(text.split())
    return normalized[:limit]


def classify_transcript_content(text: str) -> str:
    stripped = text.strip()
    if not stripped:
        return "too_short"

    non_empty_lines = [line.strip() for line in text.splitlines() if line.strip()]
    if len(non_empty_lines) == 1 and URL_ONLY_RE.match(non_empty_lines[0]):
        return "placeholder_link"
    if len(stripped) < 120 and len(non_empty_lines) < 3:
        return "too_short"
    return "valid_text"


def mega_path_depth(path: str | PurePosixPath) -> int:
    return max(len(PurePosixPath(path).parts) - 1, 0)


def extract_browser_handle(browser_url: str) -> str:
    match = FM_URL_RE.match(browser_url)
    if match is None:
        raise ValueError("MEGA browser folder URLs must look like https://mega.nz/fm/<handle>")
    return match.group(1)


def ensure_schema(connection: sqlite3.Connection) -> None:
    connection.executescript(
        """
        CREATE TABLE IF NOT EXISTS sources (
            id INTEGER PRIMARY KEY,
            browser_url TEXT UNIQUE,
            source_handle TEXT,
            canonical_path TEXT,
            current_path TEXT,
            display_name TEXT NOT NULL,
            status TEXT NOT NULL DEFAULT 'not_started',
            last_run_id TEXT,
            last_error TEXT,
            resolved_at TEXT,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS folders (
            id INTEGER PRIMARY KEY,
            path TEXT UNIQUE NOT NULL,
            parent_path TEXT,
            name TEXT NOT NULL,
            handle TEXT,
            created_at_utc TEXT NOT NULL DEFAULT '',
            depth INTEGER NOT NULL DEFAULT 0,
            media_count INTEGER NOT NULL DEFAULT 0,
            processed_media_count INTEGER NOT NULL DEFAULT 0,
            pending_media_count INTEGER NOT NULL DEFAULT 0,
            failed_media_count INTEGER NOT NULL DEFAULT 0,
            processable_count INTEGER NOT NULL DEFAULT 0,
            processed_file_count INTEGER NOT NULL DEFAULT 0,
            pending_file_count INTEGER NOT NULL DEFAULT 0,
            failed_file_count INTEGER NOT NULL DEFAULT 0,
            status TEXT NOT NULL DEFAULT 'not_started',
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS files (
            id INTEGER PRIMARY KEY,
            path TEXT UNIQUE NOT NULL,
            parent_path TEXT NOT NULL,
            basename TEXT NOT NULL,
            extension TEXT NOT NULL,
            handle TEXT,
            kind TEXT NOT NULL,
            created_at_utc TEXT,
            modified_at_utc TEXT,
            original_path TEXT,
            source_browser_url TEXT,
            source_canonical_path TEXT,
            transcript_path TEXT,
            companion_output_path TEXT,
            transcript_status TEXT,
            transcript_processor TEXT,
            transcript_content_mode TEXT,
            last_run_id TEXT,
            last_error TEXT,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS job_runs (
            id INTEGER PRIMARY KEY,
            run_id TEXT UNIQUE NOT NULL,
            source_id INTEGER,
            browser_url TEXT,
            source_path_before TEXT,
            source_path_after TEXT,
            status TEXT NOT NULL,
            processed INTEGER NOT NULL DEFAULT 0,
            skipped INTEGER NOT NULL DEFAULT 0,
            failed INTEGER NOT NULL DEFAULT 0,
            discovered INTEGER NOT NULL DEFAULT 0,
            manifest_local_path TEXT,
            manifest_s4_key TEXT,
            local_artifact_dir TEXT,
            compute_profile TEXT,
            selection_policy TEXT,
            instance_type TEXT,
            gpu_count INTEGER,
            architecture TEXT,
            image_id TEXT,
            image_family TEXT,
            image_version TEXT,
            selected_region TEXT,
            scheduler_batch_id TEXT,
            scheduler_policy TEXT,
            worker_name TEXT,
            runner_host TEXT,
            assigned_file_kinds TEXT,
            error_text TEXT,
            started_at TEXT NOT NULL,
            finished_at TEXT,
            FOREIGN KEY(source_id) REFERENCES sources(id)
        );
        """
    )
    existing_folder_columns = {row[1] for row in connection.execute("PRAGMA table_info(folders)")}
    existing_file_columns = {row[1] for row in connection.execute("PRAGMA table_info(files)")}
    existing_run_columns = {row[1] for row in connection.execute("PRAGMA table_info(job_runs)")}
    folder_column_defs = {
        "created_at_utc": "TEXT NOT NULL DEFAULT ''",
        "depth": "INTEGER NOT NULL DEFAULT 0",
        "media_count": "INTEGER NOT NULL DEFAULT 0",
        "processed_media_count": "INTEGER NOT NULL DEFAULT 0",
        "pending_media_count": "INTEGER NOT NULL DEFAULT 0",
        "failed_media_count": "INTEGER NOT NULL DEFAULT 0",
        "processable_count": "INTEGER NOT NULL DEFAULT 0",
        "processed_file_count": "INTEGER NOT NULL DEFAULT 0",
        "pending_file_count": "INTEGER NOT NULL DEFAULT 0",
        "failed_file_count": "INTEGER NOT NULL DEFAULT 0",
        "excluded": "INTEGER NOT NULL DEFAULT 0",
    }
    for column_name, definition in folder_column_defs.items():
        if column_name not in existing_folder_columns:
            connection.execute(f"ALTER TABLE folders ADD COLUMN {column_name} {definition}")
    file_column_defs = {
        "created_at_utc": "TEXT",
        "modified_at_utc": "TEXT",
        "original_path": "TEXT",
        "source_browser_url": "TEXT",
        "source_canonical_path": "TEXT",
        "companion_output_path": "TEXT",
        "transcript_processor": "TEXT",
        "transcript_content_mode": "TEXT",
        "last_run_id": "TEXT",
    }
    for column_name, definition in file_column_defs.items():
        if column_name not in existing_file_columns:
            connection.execute(f"ALTER TABLE files ADD COLUMN {column_name} {definition}")
    run_column_defs = {
        "compute_profile": "TEXT",
        "selection_policy": "TEXT",
        "instance_type": "TEXT",
        "gpu_count": "INTEGER",
        "architecture": "TEXT",
        "image_id": "TEXT",
        "image_family": "TEXT",
        "image_version": "TEXT",
        "selected_region": "TEXT",
        "scheduler_batch_id": "TEXT",
        "scheduler_policy": "TEXT",
        "worker_name": "TEXT",
        "runner_host": "TEXT",
        "assigned_file_kinds": "TEXT",
    }
    for column_name, definition in run_column_defs.items():
        if column_name not in existing_run_columns:
            connection.execute(f"ALTER TABLE job_runs ADD COLUMN {column_name} {definition}")
    connection.executescript(
        """
        CREATE INDEX IF NOT EXISTS idx_sources_status ON sources(status);
        CREATE INDEX IF NOT EXISTS idx_sources_current_path ON sources(current_path);
        CREATE INDEX IF NOT EXISTS idx_folders_status ON folders(status);
        CREATE INDEX IF NOT EXISTS idx_folders_name ON folders(name);
        CREATE INDEX IF NOT EXISTS idx_folders_created_at ON folders(created_at_utc);
        CREATE INDEX IF NOT EXISTS idx_folders_parent_path ON folders(parent_path);
        CREATE INDEX IF NOT EXISTS idx_folders_depth ON folders(depth);
        CREATE INDEX IF NOT EXISTS idx_files_kind ON files(kind);
        CREATE INDEX IF NOT EXISTS idx_files_transcript_status ON files(transcript_status);
        CREATE INDEX IF NOT EXISTS idx_files_basename ON files(basename);
        CREATE INDEX IF NOT EXISTS idx_files_parent_path ON files(parent_path);
        CREATE INDEX IF NOT EXISTS idx_files_created_at ON files(created_at_utc);
        CREATE INDEX IF NOT EXISTS idx_job_runs_status ON job_runs(status);
        """
    )
    connection.commit()


def connect_db(db_path: str | Path | None) -> sqlite3.Connection:
    resolved_path = normalize_db_path(db_path)
    resolved_path.parent.mkdir(parents=True, exist_ok=True)
    connection = sqlite3.connect(resolved_path)
    connection.row_factory = sqlite3.Row
    ensure_schema(connection)
    return connection


class MegaSshBridge:
    def __init__(self, host: str, user: str, ssh_key: str):
        self.host = host
        self.user = user
        self.ssh_key = ssh_key

    @classmethod
    def from_config(
        cls,
        profile_path: str | None = None,
        host: str | None = None,
        user: str | None = None,
        ssh_key: str | None = None,
    ) -> "MegaSshBridge":
        profile_values = parse_profile(profile_path)
        host_value = host or os.environ.get("REMOTE_HOST") or profile_values.get("REMOTE_HOST")
        user_value = user or os.environ.get("REMOTE_USER") or profile_values.get("REMOTE_USER")
        ssh_key_value = ssh_key or os.environ.get("SSH_KEY_PATH") or profile_values.get("SSH_KEY_PATH")
        if not host_value or not user_value or not ssh_key_value:
            raise RuntimeError("REMOTE_HOST, REMOTE_USER, and SSH_KEY_PATH are required for MEGA catalog access.")
        return cls(host=host_value, user=user_value, ssh_key=ssh_key_value)

    def run_mega(self, command_name: str, *args: str) -> subprocess.CompletedProcess[str]:
        remote_binary = f"/snap/bin/mega-cmd.{command_name}"
        remote_command = " ".join(shlex.quote(part) for part in [remote_binary, *args])
        wrapped_command = 'export PATH="/snap/bin:$PATH"; ' + remote_command
        ssh_command = [
            "ssh",
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "ConnectTimeout=10",
            "-i",
            self.ssh_key,
            f"{self.user}@{self.host}",
            f"bash -lc {shlex.quote(wrapped_command)}",
        ]
        startup_markers = (
            "Initiating MEGAcmd server in background",
            "Resuming session",
        )
        last_result = None
        for attempt in range(6):
            result = subprocess.run(ssh_command, capture_output=True, text=True, check=False)
            last_result = result
            combined_output = f"{result.stdout}\n{result.stderr}"
            if result.returncode == 0:
                return result
            if not any(marker in combined_output for marker in startup_markers):
                return result
            if attempt == 5:
                return result
            time.sleep(2)
        return last_result

    def resolve_browser_folder_url(self, browser_url: str) -> PurePosixPath:
        handle = extract_browser_handle(browser_url)
        result = self.run_mega("mega-ls", f"H:{handle}")
        if result.returncode != 0:
            raise RuntimeError(transcribe_mega_folder.extract_error_text(result, "mega-ls failed"))
        for raw_line in result.stdout.splitlines():
            stripped = raw_line.strip()
            if stripped.startswith("/") and stripped.endswith(":"):
                return transcribe_mega_folder.normalize_mega_path(stripped[:-1])
        raise RuntimeError(f"Unable to resolve browser folder URL: {browser_url}")

    def read_text_file(self, remote_path: str | PurePosixPath) -> str:
        result = self.run_mega("mega-cat", str(PurePosixPath(remote_path)))
        if result.returncode != 0:
            raise RuntimeError(transcribe_mega_folder.extract_error_text(result, "mega-cat failed"))
        return result.stdout

    def path_exists(self, remote_path: str | PurePosixPath) -> bool:
        normalized_path = transcribe_mega_folder.normalize_mega_path(remote_path)
        result = self.run_mega("mega-ls", str(normalized_path))
        return result.returncode == 0

    def rename_path(self, old_path: str | PurePosixPath, new_path: str | PurePosixPath) -> None:
        current_path = transcribe_mega_folder.normalize_mega_path(old_path)
        target_path = transcribe_mega_folder.normalize_mega_path(new_path)
        if current_path == target_path:
            return
        result = self.run_mega("mega-mv", str(current_path), str(target_path))
        if result.returncode == 0:
            return
        if self.path_exists(target_path) and not self.path_exists(current_path):
            return
        raise RuntimeError(transcribe_mega_folder.extract_error_text(result, "mega-mv failed"))

    def _list_tree_pass(
        self,
        root_path: str | PurePosixPath,
        *,
        recursive: bool,
        show_creation_time: bool,
    ) -> list[transcribe_mega_folder.MegaLsEntry]:
        normalized_root = transcribe_mega_folder.normalize_mega_path(root_path)
        args = ["-l"]
        if recursive:
            args.append("-R")
        args.extend(["--show-handles", f"--time-format={transcribe_mega_folder.MEGA_TIME_FORMAT}"])
        if show_creation_time:
            args.append("--show-creation-time")
        args.append(str(normalized_root))
        result = self.run_mega("mega-ls", *args)
        if result.returncode != 0:
            raise RuntimeError(transcribe_mega_folder.extract_error_text(result, "mega-ls failed"))
        return transcribe_mega_folder.parse_mega_ls_long_listing(
            result.stdout,
            normalized_root,
            file_time_field="created_at_utc" if show_creation_time else "modified_at_utc",
        )

    def list_tree(self, root_path: str | PurePosixPath) -> list[transcribe_mega_folder.MegaLsEntry]:
        normalized_root = transcribe_mega_folder.normalize_mega_path(root_path)
        modified_entries = self._list_tree_pass(normalized_root, recursive=True, show_creation_time=False)
        created_entries = self._list_tree_pass(normalized_root, recursive=True, show_creation_time=True)

        entry_map: dict[tuple[str, str], transcribe_mega_folder.MegaLsEntry] = {}
        for entry in modified_entries:
            entry_map[(entry.kind, str(entry.path))] = entry

        for entry in created_entries:
            key = (entry.kind, str(entry.path))
            existing = entry_map.get(key)
            if existing is None:
                entry_map[key] = entry
                continue
            entry_map[key] = transcribe_mega_folder.MegaLsEntry(
                path=existing.path,
                parent_path=existing.parent_path,
                name=existing.name,
                kind=existing.kind,
                handle=existing.handle or entry.handle,
                created_at_utc=existing.created_at_utc or entry.created_at_utc,
                modified_at_utc=existing.modified_at_utc or entry.modified_at_utc,
            )

        if str(normalized_root) != "/":
            parent_entries = self._list_tree_pass(normalized_root.parent, recursive=False, show_creation_time=False)
            root_entry = next((entry for entry in parent_entries if entry.path == normalized_root and entry.kind == "folder"), None)
            if root_entry is not None:
                entry_map[("folder", str(normalized_root))] = root_entry
            elif ("folder", str(normalized_root)) not in entry_map:
                entry_map[("folder", str(normalized_root))] = transcribe_mega_folder.MegaLsEntry(
                    path=normalized_root,
                    parent_path=normalized_root.parent,
                    name=normalized_root.name,
                    kind="folder",
                    handle=None,
                    created_at_utc="",
                    modified_at_utc=None,
                )

        return sorted(entry_map.values(), key=lambda entry: (str(entry.parent_path), entry.kind != "folder", entry.name))


def export_csv(connection: sqlite3.Connection, db_path: str | Path | None) -> None:
    export_dir = normalize_db_path(db_path).parent / "exports"
    export_dir.mkdir(parents=True, exist_ok=True)
    for table_name in ("sources", "folders", "files", "job_runs"):
        rows = connection.execute(f"SELECT * FROM {table_name} ORDER BY id").fetchall()
        output_path = export_dir / f"{table_name}.csv"
        with output_path.open("w", encoding="utf-8", newline="") as handle:
            writer = csv.writer(handle)
            column_names = [column[1] for column in connection.execute(f"PRAGMA table_info({table_name})")]
            writer.writerow(column_names)
            for row in rows:
                writer.writerow([row[column] for column in column_names])


def upsert_source_row(
    connection: sqlite3.Connection,
    *,
    browser_url: str | None,
    source_handle: str | None,
    canonical_path: str | None,
    current_path: str | None,
    display_name: str,
    status: str,
    last_error: str | None = None,
    last_run_id: str | None = None,
    resolved_at: str | None = None,
) -> dict:
    existing = None
    if browser_url:
        existing = connection.execute("SELECT * FROM sources WHERE browser_url = ?", (browser_url,)).fetchone()
    if existing is None and current_path:
        existing = connection.execute(
            "SELECT * FROM sources WHERE current_path = ? OR canonical_path = ?",
            (current_path, current_path),
        ).fetchone()
    source_id = existing["id"] if existing else None
    updated_at = utc_now()
    if source_id is None:
        connection.execute(
            """
            INSERT INTO sources (
                browser_url, source_handle, canonical_path, current_path, display_name,
                status, last_run_id, last_error, resolved_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                browser_url,
                source_handle,
                canonical_path,
                current_path,
                display_name,
                status,
                last_run_id,
                last_error,
                resolved_at,
                updated_at,
            ),
        )
        source_id = connection.execute("SELECT last_insert_rowid()").fetchone()[0]
    else:
        connection.execute(
            """
            UPDATE sources
            SET browser_url = COALESCE(?, browser_url),
                source_handle = COALESCE(?, source_handle),
                canonical_path = COALESCE(?, canonical_path),
                current_path = COALESCE(?, current_path),
                display_name = ?,
                status = ?,
                last_run_id = COALESCE(?, last_run_id),
                last_error = ?,
                resolved_at = COALESCE(?, resolved_at),
                updated_at = ?
            WHERE id = ?
            """,
            (
                browser_url,
                source_handle,
                canonical_path,
                current_path,
                display_name,
                status,
                last_run_id,
                last_error,
                resolved_at,
                updated_at,
                source_id,
            ),
        )
    connection.commit()
    return row_to_dict(connection.execute("SELECT * FROM sources WHERE id = ?", (source_id,)).fetchone())


def build_source_context_rows(connection: sqlite3.Connection) -> list[dict[str, str | None]]:
    rows = connection.execute(
        "SELECT browser_url, canonical_path, current_path FROM sources WHERE current_path IS NOT NULL OR canonical_path IS NOT NULL"
    ).fetchall()
    return [dict(row) for row in rows]


def find_source_context_for_path(
    path: str | PurePosixPath,
    source_context_rows: list[dict[str, str | None]],
) -> dict[str, str | None]:
    normalized_path = str(transcribe_mega_folder.normalize_mega_path(path))
    best_match: dict[str, str | None] | None = None
    best_match_length = -1

    for source_row in source_context_rows:
        for candidate_root in (source_row.get("current_path"), source_row.get("canonical_path")):
            if not candidate_root:
                continue
            if normalized_path == candidate_root or normalized_path.startswith(f"{candidate_root}/"):
                candidate_length = len(candidate_root)
                if candidate_length > best_match_length:
                    best_match = source_row
                    best_match_length = candidate_length
                break

    if best_match is None:
        return {"source_browser_url": None, "source_canonical_path": None}
    return {
        "source_browser_url": best_match.get("browser_url"),
        "source_canonical_path": best_match.get("canonical_path"),
    }


def rebuild_inventory(
    connection: sqlite3.Connection,
    entries: list[transcribe_mega_folder.MegaLsEntry],
    scope_root: str | PurePosixPath | None = None,
) -> tuple[int, int]:
    updated_at = utc_now()
    scoped_root = transcribe_mega_folder.normalize_mega_path(scope_root) if scope_root is not None else None
    if scoped_root is None:
        existing_file_rows = {
            row["path"]: row
            for row in connection.execute(
                """
                SELECT path, original_path, source_browser_url, source_canonical_path,
                       transcript_path, companion_output_path, transcript_status,
                       transcript_processor, transcript_content_mode, last_run_id, last_error
                FROM files
                """
            ).fetchall()
        }
    else:
        prefix = f"{scoped_root}/%"
        existing_file_rows = {
            row["path"]: row
            for row in connection.execute(
                """
                SELECT path, original_path, source_browser_url, source_canonical_path,
                       transcript_path, companion_output_path, transcript_status,
                       transcript_processor, transcript_content_mode, last_run_id, last_error
                FROM files
                WHERE path = ? OR path LIKE ?
                """,
                (str(scoped_root), prefix),
            ).fetchall()
        }

    excluded_paths = {
        row[0]
        for row in connection.execute("SELECT path FROM folders WHERE excluded = 1").fetchall()
    }

    if scoped_root is None:
        connection.execute("DELETE FROM folders")
        connection.execute("DELETE FROM files")
    else:
        prefix = f"{scoped_root}/%"
        connection.execute("DELETE FROM folders WHERE path = ? OR path LIKE ?", (str(scoped_root), prefix))
        connection.execute("DELETE FROM files WHERE path LIKE ?", (prefix,))
        connection.execute("DELETE FROM files WHERE path = ?", (str(scoped_root),))

    folder_entries = [entry for entry in entries if entry.kind == "folder"]
    file_entries = [entry for entry in entries if entry.kind == "file"]
    file_paths = {entry.path for entry in file_entries}
    processable_paths = candidate_processable_paths(file_paths)
    source_context_rows = build_source_context_rows(connection)

    for folder_entry in folder_entries:
        status = "transcript_done" if folder_entry.path.name.endswith(transcribe_mega_folder.MEGA_DONE_SUFFIX) else "not_started"
        excluded = 1 if str(folder_entry.path) in excluded_paths else 0
        connection.execute(
            """
            INSERT INTO folders (
                path, parent_path, name, handle, created_at_utc, depth,
                media_count, processed_media_count, pending_media_count, failed_media_count,
                processable_count, processed_file_count, pending_file_count, failed_file_count,
                status, excluded, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, 0, 0, 0, 0, 0, 0, 0, 0, ?, ?, ?)
            """,
            (
                str(folder_entry.path),
                str(folder_entry.parent_path),
                folder_entry.name,
                folder_entry.handle,
                folder_entry.created_at_utc or "",
                mega_path_depth(folder_entry.path),
                status,
                excluded,
                updated_at,
            ),
        )

    for file_entry in file_entries:
        existing_row = existing_file_rows.get(str(file_entry.path))
        source_context = find_source_context_for_path(file_entry.path, source_context_rows)
        transcript_path = expected_transcript_path(file_entry.path, file_paths, processable_paths)
        transcript_path_value = str(transcript_path) if transcript_path and transcript_path in file_paths else None
        companion_output_path = find_latest_companion_output(file_entry.path, file_paths, processable_paths)
        companion_output_value = (
            str(companion_output_path)
            if companion_output_path is not None and companion_output_path in file_paths
            else None
        )
        transcript_status = None
        transcript_processor = existing_row["transcript_processor"] if existing_row else None
        transcript_content_mode = existing_row["transcript_content_mode"] if existing_row else None
        last_run_id = existing_row["last_run_id"] if existing_row else None
        last_error = None
        original_path = existing_row["original_path"] if existing_row and existing_row["original_path"] else str(file_entry.path)
        source_browser_url = source_context["source_browser_url"] or (existing_row["source_browser_url"] if existing_row else None)
        source_canonical_path = source_context["source_canonical_path"] or (
            existing_row["source_canonical_path"] if existing_row else None
        )
        if file_entry.path in processable_paths:
            if has_paired_outputs(transcript_path_value, companion_output_value):
                if existing_row and existing_row["transcript_status"] in {"processed", "skipped_existing"}:
                    transcript_status = existing_row["transcript_status"]
                    transcript_processor = existing_row["transcript_processor"]
                    transcript_content_mode = existing_row["transcript_content_mode"]
                    last_run_id = existing_row["last_run_id"]
                elif source_canonical_path or not source_context_rows:
                    transcript_status = "processed"
                    transcript_content_mode = (
                        transcribe_mega_folder.TIMESTAMP_CONTENT_MODE
                        if classify_file_kind(file_entry.path) == "media"
                        else transcribe_mega_folder.STRUCTURED_CONTENT_MODE
                    )
                else:
                    transcript_status = "pending"
                last_error = None
            elif existing_row and existing_row["transcript_status"] == "failed":
                transcript_status = "failed"
                last_error = existing_row["last_error"]
                transcript_processor = existing_row["transcript_processor"]
                transcript_content_mode = existing_row["transcript_content_mode"]
                last_run_id = existing_row["last_run_id"]
            else:
                transcript_status = "pending"
                last_error = None
        connection.execute(
            """
            INSERT INTO files (
                path, parent_path, basename, extension, handle, kind,
                created_at_utc, modified_at_utc, original_path, source_browser_url, source_canonical_path,
                transcript_path, companion_output_path, transcript_status, transcript_processor,
                transcript_content_mode, last_run_id, last_error, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                str(file_entry.path),
                str(file_entry.parent_path),
                file_entry.name,
                file_entry.path.suffix.lower(),
                file_entry.handle,
                classify_file_kind(file_entry.path),
                file_entry.created_at_utc,
                file_entry.modified_at_utc,
                original_path,
                source_browser_url,
                source_canonical_path,
                transcript_path_value,
                companion_output_value,
                transcript_status,
                transcript_processor,
                transcript_content_mode,
                last_run_id,
                last_error,
                updated_at,
            ),
        )

    connection.commit()
    recompute_folder_statuses(connection, scope_root=scoped_root)
    mark_stale_sources(connection)
    connection.commit()
    return len(folder_entries), len(file_entries)


def recompute_folder_statuses(
    connection: sqlite3.Connection,
    scope_root: str | PurePosixPath | None = None,
) -> None:
    if scope_root is None:
        folder_rows = connection.execute("SELECT path FROM folders").fetchall()
        media_rows = connection.execute(
            "SELECT path, transcript_status FROM files WHERE kind = 'media' AND transcript_status IS NOT NULL"
        ).fetchall()
        processable_rows = connection.execute(
            "SELECT path, kind, transcript_status FROM files WHERE transcript_status IS NOT NULL"
        ).fetchall()
    else:
        normalized_root = str(transcribe_mega_folder.normalize_mega_path(scope_root))
        prefix = f"{normalized_root}/%"
        folder_rows = connection.execute(
            "SELECT path FROM folders WHERE path = ? OR path LIKE ?",
            (normalized_root, prefix),
        ).fetchall()
        media_rows = connection.execute(
            "SELECT path, transcript_status FROM files WHERE kind = 'media' AND transcript_status IS NOT NULL AND (path = ? OR path LIKE ?)",
            (normalized_root, prefix),
        ).fetchall()
        processable_rows = connection.execute(
            "SELECT path, kind, transcript_status FROM files WHERE transcript_status IS NOT NULL AND (path = ? OR path LIKE ?)",
            (normalized_root, prefix),
        ).fetchall()

    media_records = [(row["path"], row["transcript_status"]) for row in media_rows]
    processable_records = [(row["path"], row["kind"], row["transcript_status"]) for row in processable_rows]
    for folder_row in folder_rows:
        folder_path = folder_row["path"]
        relevant = [
            record
            for record in media_records
            if record[0] == folder_path or record[0].startswith(f"{folder_path}/")
        ]
        processable_relevant = [
            record
            for record in processable_records
            if record[0] == folder_path or record[0].startswith(f"{folder_path}/")
        ]
        media_count = len(relevant)
        processed_media_count = sum(1 for _, status in relevant if status in {"processed", "skipped_existing"})
        failed_media_count = sum(1 for _, status in relevant if status == "failed")
        pending_media_count = max(media_count - processed_media_count - failed_media_count, 0)
        processable_count = len(processable_relevant)
        processed_file_count = sum(
            1 for _, _, status in processable_relevant if status in {"processed", "skipped_existing"}
        )
        failed_file_count = sum(1 for _, _, status in processable_relevant if status == "failed")
        pending_file_count = max(processable_count - processed_file_count - failed_file_count, 0)

        if PurePosixPath(folder_path).name.endswith(transcribe_mega_folder.MEGA_DONE_SUFFIX):
            status = "transcript_done"
        elif not processable_relevant:
            status = "not_started"
        elif any(record[2] == "failed" for record in processable_relevant):
            status = "partial_failed"
        elif all(record[2] in {"processed", "skipped_existing"} for record in processable_relevant):
            status = "transcript_done"
        else:
            status = "in_progress"
        connection.execute(
            """
            UPDATE folders
            SET status = ?,
                media_count = ?,
                processed_media_count = ?,
                pending_media_count = ?,
                failed_media_count = ?,
                processable_count = ?,
                processed_file_count = ?,
                pending_file_count = ?,
                failed_file_count = ?
            WHERE path = ?
            """,
            (
                status,
                media_count,
                processed_media_count,
                pending_media_count,
                failed_media_count,
                processable_count,
                processed_file_count,
                pending_file_count,
                failed_file_count,
                folder_path,
            ),
        )


def mark_stale_sources(connection: sqlite3.Connection) -> None:
    folder_paths = {row["path"] for row in connection.execute("SELECT path FROM folders")}
    for source_row in connection.execute("SELECT id, current_path, status FROM sources").fetchall():
        current_path = source_row["current_path"]
        if not current_path:
            continue
        if current_path not in folder_paths and source_row["status"] not in {"running", "needs_source_resolution"}:
            connection.execute("UPDATE sources SET status = ? WHERE id = ?", ("stale", source_row["id"]))


def sync_account(
    db_path: str | Path | None,
    bridge: MegaSshBridge | FakeMegaBridge,
) -> dict:
    entries = bridge.list_tree("/")
    with connect_db(db_path) as connection:
        folder_count, file_count = rebuild_inventory(connection, entries)
        export_csv(connection, db_path)
    return {"folders": folder_count, "files": file_count}


def sync_source(
    db_path: str | Path | None,
    bridge: MegaSshBridge | FakeMegaBridge,
    source_path: str | PurePosixPath,
) -> dict:
    normalized_path = transcribe_mega_folder.normalize_mega_path(source_path)
    entries = list(bridge.list_tree(normalized_path))
    if not any(entry.path == normalized_path and entry.kind == "folder" for entry in entries):
        entries.insert(
            0,
            transcribe_mega_folder.MegaLsEntry(
                path=normalized_path,
                parent_path=normalized_path.parent,
                name=normalized_path.name,
                kind="folder",
                handle=None,
                created_at_utc="",
                modified_at_utc=None,
            ),
        )
    with connect_db(db_path) as connection:
        folder_count, file_count = rebuild_inventory(connection, entries, scope_root=normalized_path)
        export_csv(connection, db_path)
    return {"folders": folder_count, "files": file_count, "source_path": str(normalized_path)}


def resolve_source(
    db_path: str | Path | None,
    bridge: MegaSshBridge | FakeMegaBridge,
    browser_url: str,
) -> dict:
    source_handle = extract_browser_handle(browser_url)
    resolved_at = utc_now()
    try:
        canonical_path = str(bridge.resolve_browser_folder_url(browser_url))
        display_name = PurePosixPath(canonical_path).name
        status = "not_started"
        last_error = None
    except Exception as exc:  # pragma: no cover - exercised in tests via fakes.
        canonical_path = None
        display_name = source_handle
        status = "needs_source_resolution"
        last_error = str(exc)

    with connect_db(db_path) as connection:
        source_row = upsert_source_row(
            connection,
            browser_url=browser_url,
            source_handle=source_handle,
            canonical_path=canonical_path,
            current_path=canonical_path,
            display_name=display_name,
            status=status,
            last_error=last_error,
            resolved_at=resolved_at if canonical_path else None,
        )
        export_csv(connection, db_path)
    return source_row


def find_source_row(connection: sqlite3.Connection, identifier: str) -> sqlite3.Row | None:
    return connection.execute(
        """
        SELECT * FROM sources
        WHERE browser_url = ? OR canonical_path = ? OR current_path = ? OR display_name = ?
        """,
        (identifier, identifier, identifier, identifier),
    ).fetchone()


def show_source(db_path: str | Path | None, identifier: str) -> dict | None:
    with connect_db(db_path) as connection:
        return row_to_dict(find_source_row(connection, identifier))


def get_folder(db_path: str | Path | None, path: str | PurePosixPath) -> dict | None:
    normalized_path = str(transcribe_mega_folder.normalize_mega_path(path))
    with connect_db(db_path) as connection:
        row = connection.execute("SELECT * FROM folders WHERE path = ?", (normalized_path,)).fetchone()
        return row_to_dict(row)


def get_file(db_path: str | Path | None, path: str | PurePosixPath) -> dict | None:
    normalized_path = str(transcribe_mega_folder.normalize_mega_path(path))
    with connect_db(db_path) as connection:
        row = connection.execute("SELECT * FROM files WHERE path = ?", (normalized_path,)).fetchone()
        return row_to_dict(row)


def list_sources(
    db_path: str | Path | None,
    *,
    status: str | None = None,
    limit: int | None = 50,
    sort: str = "updated_desc",
) -> list[dict]:
    order_by = {
        "updated_desc": "updated_at DESC, display_name ASC",
        "updated_asc": "updated_at ASC, display_name ASC",
        "name": "display_name ASC, updated_at DESC",
        "status": "status ASC, display_name ASC",
    }
    if sort not in order_by:
        raise ValueError(f"Unsupported source sort: {sort}")

    clauses = []
    parameters: list[object] = []
    if status is not None:
        clauses.append("status = ?")
        parameters.append(status)

    sql = "SELECT * FROM sources"
    if clauses:
        sql += " WHERE " + " AND ".join(clauses)
    sql += f" ORDER BY {order_by[sort]}"
    if limit is not None:
        sql += " LIMIT ?"
        parameters.append(limit)

    with connect_db(db_path) as connection:
        rows = connection.execute(sql, parameters).fetchall()
        return [dict(row) for row in rows]


def list_folders(
    db_path: str | Path | None,
    status: str | None = None,
    *,
    created_from: str | None = None,
    created_to: str | None = None,
    sort: str = "path",
    limit: int | None = None,
    top_level_only: bool = False,
    pending_transcription_only: bool = False,
) -> list[dict]:
    order_by = {
        "path": "path ASC",
        "created_desc": "created_at_utc DESC, path ASC",
        "created_asc": "created_at_utc ASC, path ASC",
        "name": "name ASC, path ASC",
        "media_desc": "CAST(media_count AS INTEGER) DESC, path ASC",
        "processed_desc": "CAST(processed_media_count AS INTEGER) DESC, path ASC",
        "pending_desc": "CAST(pending_media_count AS INTEGER) DESC, path ASC",
        "failed_desc": "CAST(failed_media_count AS INTEGER) DESC, path ASC",
    }
    if sort not in order_by:
        raise ValueError(f"Unsupported folder sort: {sort}")

    clauses = []
    parameters: list[object] = []
    if status is not None:
        clauses.append("status = ?")
        parameters.append(status)
    if created_from is not None:
        clauses.append("created_at_utc >= ?")
        parameters.append(created_from)
    if created_to is not None:
        clauses.append("created_at_utc < ?")
        parameters.append(created_to)
    if top_level_only:
        clauses.append("depth = 1")
    if pending_transcription_only:
        clauses.append("(CAST(pending_media_count AS INTEGER) > 0 OR CAST(pending_file_count AS INTEGER) > 0)")
    clauses.append("COALESCE(excluded, 0) = 0")

    sql = "SELECT * FROM folders"
    if clauses:
        sql += " WHERE " + " AND ".join(clauses)
    sql += f" ORDER BY {order_by[sort]}"
    if limit is not None:
        sql += " LIMIT ?"
        parameters.append(limit)

    with connect_db(db_path) as connection:
        rows = connection.execute(sql, parameters).fetchall()
        return [dict(row) for row in rows]


def list_files(
    db_path: str | Path | None,
    *,
    folder_path: str,
    sort: str = "created_desc",
    limit: int | None = None,
    kind: str | None = None,
    transcript_status: str | None = None,
) -> list[dict]:
    normalized_folder = str(transcribe_mega_folder.normalize_mega_path(folder_path))
    order_by = {
        "created_desc": "created_at_utc DESC, path ASC",
        "modified_desc": "modified_at_utc DESC, path ASC",
        "name": "basename ASC, path ASC",
    }
    if sort not in order_by:
        raise ValueError(f"Unsupported file sort: {sort}")

    clauses = ["(path LIKE ? OR path = ?)"]
    parameters: list[object] = [f"{normalized_folder}/%", normalized_folder]
    if kind is not None:
        clauses.append("kind = ?")
        parameters.append(kind)
    if transcript_status is not None:
        clauses.append("transcript_status = ?")
        parameters.append(transcript_status)

    sql = f"SELECT * FROM files WHERE {' AND '.join(clauses)} ORDER BY {order_by[sort]}"
    if limit is not None:
        sql += " LIMIT ?"
        parameters.append(limit)

    with connect_db(db_path) as connection:
        rows = connection.execute(sql, parameters).fetchall()
        return [dict(row) for row in rows]


def list_transcript_candidates(
    db_path: str | Path | None,
    *,
    limit: int | None = None,
) -> list[dict]:
    sql = """
        SELECT path, transcript_path, companion_output_path, transcript_status, basename, parent_path, updated_at
        FROM files
        WHERE transcript_path IS NOT NULL
          AND transcript_status IN ('processed', 'skipped_existing')
        ORDER BY updated_at DESC, path ASC
    """
    parameters: list[object] = []
    if limit is not None:
        sql += " LIMIT ?"
        parameters.append(limit)

    with connect_db(db_path) as connection:
        rows = connection.execute(sql, parameters).fetchall()
        return [dict(row) for row in rows]


def mega_path_is_within(path: str | PurePosixPath, root: str | PurePosixPath) -> bool:
    normalized_path = transcribe_mega_folder.normalize_mega_path(path)
    normalized_root = transcribe_mega_folder.normalize_mega_path(root)
    return normalized_path == normalized_root or str(normalized_path).startswith(f"{normalized_root}/")


def build_local_transcript_path(
    output_root: str | Path,
    transcript_path: str | PurePosixPath,
) -> Path:
    normalized_transcript_path = transcribe_mega_folder.normalize_mega_path(transcript_path)
    return Path(output_root) / str(normalized_transcript_path).lstrip("/")


def normalize_failure_error(error_text: str | None) -> str:
    if not error_text:
        return "unknown failure"
    normalized = MEGA_ERR_PREFIX_RE.sub("", error_text.strip())
    return normalized or "unknown failure"


def filter_transcript_candidates(
    candidates: list[dict],
    *,
    folder_path: str | PurePosixPath | None = None,
) -> list[dict]:
    if folder_path is None:
        return list(candidates)
    return [
        candidate
        for candidate in candidates
        if mega_path_is_within(candidate["path"], folder_path)
    ]


def verify_transcripts(
    db_path: str | Path | None,
    bridge: MegaSshBridge | FakeMegaBridge,
    *,
    output_root: str | Path = DEFAULT_LOCAL_TRANSCRIPT_DIR,
    folder_path: str | PurePosixPath | None = None,
    limit: int | None = None,
    validate_content: bool = False,
) -> dict:
    candidates = filter_transcript_candidates(
        list_transcript_candidates(db_path, limit=limit),
        folder_path=folder_path,
    )
    items: list[dict] = []
    available_on_mega = 0
    available_locally = 0
    missing_on_mega = 0
    missing_locally = 0
    content_validation_summary = {
        "valid_text": 0,
        "placeholder_link": 0,
        "too_short": 0,
    }

    for candidate in candidates:
        transcript_path = candidate["transcript_path"]
        companion_output_path = candidate.get("companion_output_path")
        local_path = build_local_transcript_path(output_root, transcript_path)
        local_exists = local_path.exists()
        mega_exists = False
        companion_local_path = (
            build_local_transcript_path(output_root, companion_output_path)
            if companion_output_path
            else None
        )
        companion_exists_locally = companion_local_path.exists() if companion_local_path is not None else False
        companion_exists_on_mega = False
        error = None
        content_validation = None
        validated_from = None
        preview = None
        try:
            mega_exists = bool(bridge.path_exists(transcript_path))
        except Exception as exc:  # pragma: no cover - exercised through fakes.
            error = str(exc)
        if companion_output_path and error is None:
            try:
                companion_exists_on_mega = bool(bridge.path_exists(companion_output_path))
            except Exception as exc:  # pragma: no cover - exercised through fakes.
                error = str(exc)

        if mega_exists:
            available_on_mega += 1
        else:
            missing_on_mega += 1

        if local_exists:
            available_locally += 1
        else:
            missing_locally += 1

        if validate_content and mega_exists and error is None:
            try:
                remote_text = bridge.read_text_file(transcript_path)
            except Exception as exc:  # pragma: no cover - exercised through fakes.
                error = str(exc)
            else:
                content_validation = classify_transcript_content(remote_text)
                content_validation_summary[content_validation] += 1
                validated_from = "mega"
                preview = build_transcript_preview(remote_text)

        items.append(
            {
                "source_path": candidate["path"],
                "transcript_path": transcript_path,
                "exists_on_mega": mega_exists,
                "exists_locally": local_exists,
                "local_path": str(local_path),
                "companion_output_path": companion_output_path,
                "companion_exists_on_mega": companion_exists_on_mega,
                "companion_exists_locally": companion_exists_locally,
                "companion_local_path": str(companion_local_path) if companion_local_path is not None else None,
                "error": error,
                "content_validation": content_validation,
                "validated_from": validated_from,
                "preview": preview,
            }
        )

    result = {
        "verified": len(candidates),
        "available_on_mega": available_on_mega,
        "available_locally": available_locally,
        "missing_on_mega": missing_on_mega,
        "missing_locally": missing_locally,
        "items": items,
    }
    if validate_content:
        result["content_validation_summary"] = content_validation_summary
    return result


def sync_transcripts(
    db_path: str | Path | None,
    bridge: MegaSshBridge | FakeMegaBridge,
    *,
    output_root: str | Path = DEFAULT_LOCAL_TRANSCRIPT_DIR,
    folder_path: str | PurePosixPath | None = None,
    limit: int | None = None,
    force: bool = False,
) -> dict:
    candidates = filter_transcript_candidates(
        list_transcript_candidates(db_path, limit=limit),
        folder_path=folder_path,
    )
    downloaded = 0
    skipped_existing = 0
    missing_on_mega = 0
    failed = 0
    items: list[dict] = []

    for candidate in candidates:
        transcript_path = candidate["transcript_path"]
        companion_output_path = candidate.get("companion_output_path")
        local_path = build_local_transcript_path(output_root, transcript_path)
        companion_local_path = (
            build_local_transcript_path(output_root, companion_output_path)
            if companion_output_path
            else None
        )
        item = {
            "source_path": candidate["path"],
            "transcript_path": transcript_path,
            "local_path": str(local_path),
            "companion_output_path": companion_output_path,
            "companion_local_path": str(companion_local_path) if companion_local_path is not None else None,
            "status": None,
            "error": None,
        }
        if (
            local_path.exists()
            and not force
            and (companion_output_path is None or (companion_local_path is not None and companion_local_path.exists()))
        ):
            skipped_existing += 1
            item["status"] = "skipped_existing"
            items.append(item)
            continue

        try:
            if not bridge.path_exists(transcript_path):
                missing_on_mega += 1
                item["status"] = "missing_on_mega"
                items.append(item)
                continue
            content = bridge.read_text_file(transcript_path)
            local_path.parent.mkdir(parents=True, exist_ok=True)
            local_path.write_text(content, encoding="utf-8")
            if companion_output_path is not None and companion_local_path is not None and bridge.path_exists(companion_output_path):
                companion_content = bridge.read_text_file(companion_output_path)
                companion_local_path.parent.mkdir(parents=True, exist_ok=True)
                companion_local_path.write_text(companion_content, encoding="utf-8")
        except Exception as exc:  # pragma: no cover - exercised through fakes.
            failed += 1
            item["status"] = "failed"
            item["error"] = str(exc)
        else:
            downloaded += 1
            item["status"] = "downloaded"
        items.append(item)

    return {
        "considered": len(candidates),
        "downloaded": downloaded,
        "skipped_existing": skipped_existing,
        "missing_on_mega": missing_on_mega,
        "failed": failed,
        "output_root": str(Path(output_root)),
        "items": items,
    }


def analyze_failures(
    db_path: str | Path | None,
    *,
    run_limit: int | None = 10,
    file_limit: int | None = 20,
) -> dict:
    with connect_db(db_path) as connection:
        catalog_failed_files = connection.execute(
            "SELECT COUNT(*) FROM files WHERE transcript_status = 'failed'"
        ).fetchone()[0]
        failed_file_rows = connection.execute(
            """
            SELECT path, transcript_path, last_error, updated_at
            FROM files
            WHERE transcript_status = 'failed'
            ORDER BY updated_at DESC, path ASC
            LIMIT ?
            """,
            (file_limit,),
        ).fetchall()
        recent_run_rows = connection.execute(
            """
            SELECT run_id, source_path_before, source_path_after, status, failed, discovered,
                   manifest_local_path, error_text, started_at, finished_at
            FROM job_runs
            WHERE status = 'partial_failed'
            ORDER BY started_at DESC, run_id DESC
            LIMIT ?
            """,
            (run_limit,),
        ).fetchall()

    error_counts: dict[str, int] = {}
    runs: list[dict] = []
    for row in recent_run_rows:
        manifest_errors: list[str] = []
        failed_items = int(row["failed"] or 0)
        manifest_path = row["manifest_local_path"]
        if manifest_path and Path(manifest_path).exists():
            try:
                manifest = json.loads(Path(manifest_path).read_text(encoding="utf-8"))
            except Exception as exc:  # pragma: no cover - defensive parsing path.
                manifest_errors.append(f"manifest parse failed: {exc}")
            else:
                failed_manifest_items = [
                    item for item in manifest.get("items", []) if item.get("status") == "failed"
                ]
                failed_items = len(failed_manifest_items)
                manifest_errors.extend(
                    normalize_failure_error(item.get("error"))
                    for item in failed_manifest_items
                )
        elif row["error_text"]:
            manifest_errors.append(normalize_failure_error(row["error_text"]))

        for error in manifest_errors:
            error_counts[error] = error_counts.get(error, 0) + 1

        runs.append(
            {
                "run_id": row["run_id"],
                "source_path_before": row["source_path_before"],
                "source_path_after": row["source_path_after"],
                "failed_items": failed_items,
                "discovered": row["discovered"],
                "manifest_local_path": manifest_path,
                "error_text": row["error_text"],
                "started_at": row["started_at"],
                "finished_at": row["finished_at"],
            }
        )

    top_errors = [
        {"error": error, "count": count}
        for error, count in sorted(error_counts.items(), key=lambda item: (-item[1], item[0]))
    ]

    return {
        "catalog_failed_files": catalog_failed_files,
        "recent_partial_failed_runs": len(runs),
        "top_errors": top_errors,
        "runs": runs,
        "failed_files": [dict(row) for row in failed_file_rows],
    }


def build_knowledge_base(
    *,
    kb_db_path: str | Path | None = None,
    transcriptions_root: str | Path | None = None,
    qdrant_path: str | Path | None = None,
    catalog_db_path: str | Path | None = None,
    vector_store=None,
    embedder=None,
):
    try:
        import transcript_kb
    except ModuleNotFoundError:  # pragma: no cover - import path differs between script and package usage.
        from tools import transcript_kb

    return transcript_kb.TranscriptKnowledgeBase(
        db_path=Path(kb_db_path) if kb_db_path is not None else DEFAULT_KB_DB_PATH,
        transcriptions_root=Path(transcriptions_root) if transcriptions_root is not None else DEFAULT_LOCAL_TRANSCRIPT_DIR,
        qdrant_path=Path(qdrant_path) if qdrant_path is not None else DEFAULT_KB_QDRANT_PATH,
        catalog_db_path=Path(catalog_db_path) if catalog_db_path is not None else normalize_db_path(DEFAULT_DB_PATH),
        vector_store=vector_store,
        embedder=embedder,
    )


def kb_sync(
    *,
    kb_db_path: str | Path | None = None,
    transcriptions_root: str | Path | None = None,
    qdrant_path: str | Path | None = None,
    catalog_db_path: str | Path | None = None,
    vector_store=None,
    embedder=None,
    force: bool = False,
) -> dict:
    kb = build_knowledge_base(
        kb_db_path=kb_db_path,
        transcriptions_root=transcriptions_root,
        qdrant_path=qdrant_path,
        catalog_db_path=catalog_db_path,
        vector_store=vector_store,
        embedder=embedder,
    )
    return kb.sync(force=force)


def kb_status(
    *,
    kb_db_path: str | Path | None = None,
    transcriptions_root: str | Path | None = None,
    qdrant_path: str | Path | None = None,
    catalog_db_path: str | Path | None = None,
    vector_store=None,
    embedder=None,
) -> dict:
    kb = build_knowledge_base(
        kb_db_path=kb_db_path,
        transcriptions_root=transcriptions_root,
        qdrant_path=qdrant_path,
        catalog_db_path=catalog_db_path,
        vector_store=vector_store,
        embedder=embedder,
    )
    return kb.status()


def kb_query(
    query: str,
    *,
    kb_db_path: str | Path | None = None,
    transcriptions_root: str | Path | None = None,
    qdrant_path: str | Path | None = None,
    catalog_db_path: str | Path | None = None,
    vector_store=None,
    embedder=None,
    limit: int = 5,
    course_name: str | None = None,
    module_path: str | None = None,
    content_type: str | None = None,
    has_timestamps: bool | None = None,
    synthesize: bool = False,
) -> dict:
    kb = build_knowledge_base(
        kb_db_path=kb_db_path,
        transcriptions_root=transcriptions_root,
        qdrant_path=qdrant_path,
        catalog_db_path=catalog_db_path,
        vector_store=vector_store,
        embedder=embedder,
    )
    return kb.query(
        query,
        limit=limit,
        course_name=course_name,
        module_path=module_path,
        content_type=content_type,
        has_timestamps=has_timestamps,
        synthesize=synthesize,
    )


def search_files(
    db_path: str | Path | None,
    query: str,
    *,
    kind: str | None = None,
    transcript_status: str | None = None,
) -> list[dict]:
    sql = "SELECT * FROM files WHERE (path LIKE ? OR basename LIKE ?)"
    parameters: list[str] = [f"%{query}%", f"%{query}%"]
    if kind is not None:
        sql += " AND kind = ?"
        parameters.append(kind)
    if transcript_status is not None:
        sql += " AND transcript_status = ?"
        parameters.append(transcript_status)
    sql += " ORDER BY path"
    with connect_db(db_path) as connection:
        rows = connection.execute(sql, parameters).fetchall()
        return [dict(row) for row in rows]


def list_job_runs(
    db_path: str | Path | None,
    *,
    status: str | None = None,
    limit: int | None = 50,
    sort: str = "started_desc",
) -> list[dict]:
    order_by = {
        "started_desc": "started_at DESC, run_id DESC",
        "started_asc": "started_at ASC, run_id ASC",
        "status": "status ASC, started_at DESC",
    }
    if sort not in order_by:
        raise ValueError(f"Unsupported job run sort: {sort}")

    clauses = []
    parameters: list[object] = []
    if status is not None:
        clauses.append("status = ?")
        parameters.append(status)

    sql = "SELECT * FROM job_runs"
    if clauses:
        sql += " WHERE " + " AND ".join(clauses)
    sql += f" ORDER BY {order_by[sort]}"
    if limit is not None:
        sql += " LIMIT ?"
        parameters.append(limit)

    with connect_db(db_path) as connection:
        rows = connection.execute(sql, parameters).fetchall()
        return [dict(row) for row in rows]


def get_job_run(db_path: str | Path | None, run_id: str) -> dict | None:
    with connect_db(db_path) as connection:
        row = connection.execute("SELECT * FROM job_runs WHERE run_id = ?", (run_id,)).fetchone()
        return row_to_dict(row)


def get_dashboard_summary(db_path: str | Path | None) -> dict:
    with connect_db(db_path) as connection:
        total_folders = connection.execute("SELECT COUNT(*) FROM folders WHERE depth = 1").fetchone()[0]
        total_files = connection.execute("SELECT COUNT(*) FROM files").fetchone()[0]
        total_sources = connection.execute("SELECT COUNT(*) FROM sources").fetchone()[0]
        total_runs = connection.execute("SELECT COUNT(*) FROM job_runs").fetchone()[0]
        processed_transcripts = connection.execute(
            "SELECT COUNT(*) FROM files WHERE transcript_status IN ('processed', 'skipped_existing')"
        ).fetchone()[0]
        pending_transcripts = connection.execute(
            "SELECT COUNT(*) FROM files WHERE transcript_status = 'pending'"
        ).fetchone()[0]
        failed_transcripts = connection.execute(
            "SELECT COUNT(*) FROM files WHERE transcript_status = 'failed'"
        ).fetchone()[0]
        folder_status_rows = connection.execute(
            "SELECT status, COUNT(*) AS count FROM folders WHERE depth = 1 GROUP BY status ORDER BY status"
        ).fetchall()
        source_status_rows = connection.execute(
            "SELECT status, COUNT(*) AS count FROM sources GROUP BY status ORDER BY status"
        ).fetchall()

    return {
        "total_folders": total_folders,
        "total_files": total_files,
        "total_sources": total_sources,
        "total_runs": total_runs,
        "processed_transcripts": processed_transcripts,
        "pending_transcripts": pending_transcripts,
        "failed_transcripts": failed_transcripts,
        "folder_status_counts": {row["status"]: row["count"] for row in folder_status_rows},
        "source_status_counts": {row["status"]: row["count"] for row in source_status_rows},
    }


def start_run(
    db_path: str | Path | None,
    *,
    source_path: str,
    run_id: str,
    browser_url: str | None = None,
    scheduler_batch_id: str | None = None,
    scheduler_policy: str | None = None,
    worker_name: str | None = None,
    runner_host: str | None = None,
    assigned_file_kinds: str | None = None,
) -> dict:
    normalized_source = str(transcribe_mega_folder.normalize_mega_path(source_path))
    display_name = PurePosixPath(normalized_source).name
    started_at = utc_now()
    with connect_db(db_path) as connection:
        source_row = upsert_source_row(
            connection,
            browser_url=browser_url,
            source_handle=extract_browser_handle(browser_url) if browser_url else None,
            canonical_path=normalized_source,
            current_path=normalized_source,
            display_name=display_name,
            status="running",
            last_run_id=run_id,
            last_error=None,
        )
        connection.execute(
            """
            INSERT OR REPLACE INTO job_runs (
                run_id, source_id, browser_url, source_path_before, source_path_after, status,
                processed, skipped, failed, discovered, manifest_local_path, manifest_s4_key,
                local_artifact_dir, compute_profile, selection_policy, instance_type, gpu_count,
                architecture, image_id, image_family, image_version, selected_region,
                scheduler_batch_id, scheduler_policy, worker_name, runner_host, assigned_file_kinds,
                error_text, started_at, finished_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                run_id,
                source_row["id"],
                browser_url,
                normalized_source,
                normalized_source,
                "running",
                0,
                0,
                0,
                0,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                scheduler_batch_id,
                scheduler_policy,
                worker_name,
                runner_host,
                assigned_file_kinds,
                None,
                started_at,
                None,
            ),
        )
        connection.execute(
            "UPDATE folders SET status = ? WHERE path = ?",
            ("running", normalized_source),
        )
        connection.commit()
        export_csv(connection, db_path)
        return row_to_dict(connection.execute("SELECT * FROM job_runs WHERE run_id = ?", (run_id,)).fetchone())


def rewrite_paths(
    connection: sqlite3.Connection,
    old_root: str,
    new_root: str,
) -> None:
    def rewrite(value: str | None) -> str | None:
        if value is None:
            return None
        if value == old_root:
            return new_root
        if value.startswith(f"{old_root}/"):
            return new_root + value[len(old_root):]
        return value

    folder_rows = connection.execute(
        "SELECT id, path, parent_path FROM folders WHERE path = ? OR path LIKE ?",
        (old_root, f"{old_root}/%"),
    ).fetchall()
    for row in folder_rows:
        connection.execute(
            "UPDATE folders SET path = ?, parent_path = ? WHERE id = ?",
            (rewrite(row["path"]), rewrite(row["parent_path"]), row["id"]),
        )

    file_rows = connection.execute(
        """
        SELECT id, path, parent_path, transcript_path, companion_output_path
        FROM files
        WHERE path = ? OR path LIKE ?
        """,
        (old_root, f"{old_root}/%"),
    ).fetchall()
    for row in file_rows:
        connection.execute(
            """
            UPDATE files
            SET path = ?,
                parent_path = ?,
                transcript_path = ?,
                companion_output_path = ?
            WHERE id = ?
            """,
            (
                rewrite(row["path"]),
                rewrite(row["parent_path"]),
                rewrite(row["transcript_path"]),
                rewrite(row["companion_output_path"]),
                row["id"],
            ),
        )

    source_rows = connection.execute(
        "SELECT id, current_path FROM sources WHERE current_path = ? OR current_path LIKE ?",
        (old_root, f"{old_root}/%"),
    ).fetchall()
    for row in source_rows:
        connection.execute(
            "UPDATE sources SET current_path = ? WHERE id = ?",
            (rewrite(row["current_path"]), row["id"]),
        )

    run_rows = connection.execute(
        "SELECT id, source_path_after FROM job_runs WHERE source_path_after = ? OR source_path_after LIKE ?",
        (old_root, f"{old_root}/%"),
    ).fetchall()
    for row in run_rows:
        connection.execute(
            "UPDATE job_runs SET source_path_after = ? WHERE id = ?",
            (rewrite(row["source_path_after"]), row["id"]),
        )


def update_manifest_item_statuses(connection: sqlite3.Connection, manifest: dict, *, run_id: str | None = None) -> None:
    source_context_rows = build_source_context_rows(connection)
    for item in manifest.get("items", []):
        source_path = item["source"]
        if "::" in source_path:
            continue
        existing_file_row = connection.execute(
            """
            SELECT transcript_path, companion_output_path, transcript_status, transcript_processor,
                   transcript_content_mode, last_run_id, last_error
            FROM files
            WHERE path = ?
            """,
            (source_path,),
        ).fetchone()
        output_path = item["output_path"]
        companion_output_path = item.get("companion_output_path")
        source_context = find_source_context_for_path(source_path, source_context_rows)
        item_status = item["status"]
        existing_completed_pair = bool(
            existing_file_row is not None
            and existing_file_row["transcript_status"] in {"processed", "skipped_existing"}
            and has_paired_outputs(
                existing_file_row["transcript_path"],
                existing_file_row["companion_output_path"],
            )
        )
        if item_status == "failed" and existing_completed_pair:
            transcript_status = existing_file_row["transcript_status"]
            stored_output_path = existing_file_row["transcript_path"]
            stored_companion_output_path = existing_file_row["companion_output_path"]
            stored_processor = existing_file_row["transcript_processor"]
            stored_content_mode = existing_file_row["transcript_content_mode"]
            stored_last_run_id = existing_file_row["last_run_id"]
            stored_last_error = existing_file_row["last_error"]
        else:
            transcript_status = {
                "processed": "processed",
                "skipped": "skipped_existing",
                "failed": "failed",
            }[item_status]
            stored_output_path = output_path if item_status != "failed" else None
            stored_companion_output_path = (
                companion_output_path
                if item_status in {"processed", "skipped"}
                else None
            )
            stored_processor = item.get("processor")
            stored_content_mode = item.get("content_mode")
            stored_last_run_id = run_id
            stored_last_error = item.get("error")
        connection.execute(
            """
            INSERT INTO files (
                path, parent_path, basename, extension, handle, kind,
                original_path, source_browser_url, source_canonical_path,
                transcript_path, companion_output_path, transcript_status, transcript_processor,
                transcript_content_mode, last_run_id, last_error, updated_at
            ) VALUES (?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(path) DO UPDATE SET
                original_path = COALESCE(files.original_path, excluded.original_path),
                source_browser_url = COALESCE(excluded.source_browser_url, files.source_browser_url),
                source_canonical_path = COALESCE(excluded.source_canonical_path, files.source_canonical_path),
                transcript_path = excluded.transcript_path,
                companion_output_path = excluded.companion_output_path,
                transcript_status = excluded.transcript_status,
                transcript_processor = excluded.transcript_processor,
                transcript_content_mode = excluded.transcript_content_mode,
                last_run_id = excluded.last_run_id,
                last_error = excluded.last_error,
                updated_at = excluded.updated_at
            """,
            (
                source_path,
                str(PurePosixPath(source_path).parent),
                PurePosixPath(source_path).name,
                PurePosixPath(source_path).suffix.lower(),
                classify_file_kind(source_path),
                source_path,
                source_context["source_browser_url"],
                source_context["source_canonical_path"],
                stored_output_path,
                stored_companion_output_path,
                transcript_status,
                stored_processor,
                stored_content_mode,
                stored_last_run_id,
                stored_last_error,
                utc_now(),
            ),
        )
        if item_status != "failed":
            connection.execute(
                """
                INSERT OR IGNORE INTO files (
                    path, parent_path, basename, extension, handle, kind,
                    original_path, source_browser_url, source_canonical_path,
                    transcript_path, companion_output_path, transcript_status, transcript_processor,
                    transcript_content_mode, last_run_id, last_error, updated_at
                ) VALUES (?, ?, ?, ?, NULL, ?, ?, ?, ?, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?)
                """,
                (
                    output_path,
                    str(PurePosixPath(output_path).parent),
                    PurePosixPath(output_path).name,
                    PurePosixPath(output_path).suffix.lower(),
                    classify_file_kind(output_path),
                    output_path,
                    source_context["source_browser_url"],
                    source_context["source_canonical_path"],
                    utc_now(),
                ),
            )
        if item_status in {"processed", "skipped"} and companion_output_path:
            connection.execute(
                """
                INSERT OR IGNORE INTO files (
                    path, parent_path, basename, extension, handle, kind,
                    original_path, source_browser_url, source_canonical_path,
                    transcript_path, companion_output_path, transcript_status, transcript_processor,
                    transcript_content_mode, last_run_id, last_error, updated_at
                ) VALUES (?, ?, ?, ?, NULL, ?, ?, ?, ?, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?)
                """,
                (
                    companion_output_path,
                    str(PurePosixPath(companion_output_path).parent),
                    PurePosixPath(companion_output_path).name,
                    PurePosixPath(companion_output_path).suffix.lower(),
                    classify_file_kind(companion_output_path),
                    companion_output_path,
                    source_context["source_browser_url"],
                    source_context["source_canonical_path"],
                    utc_now(),
                ),
            )


def ingest_run(
    db_path: str | Path | None,
    bridge: MegaSshBridge | FakeMegaBridge | None,
    *,
    run_id: str,
    manifest_path: str | Path,
    manifest_s4_key: str | None = None,
) -> dict:
    manifest = json.loads(Path(manifest_path).read_text(encoding="utf-8"))
    source_path_before = manifest.get("source_path_before") or manifest.get("source")
    source_path_after = manifest.get("source_path_after") or source_path_before
    run_status = "transcript_done" if manifest.get("status") == "completed" else "partial_failed"

    with connect_db(db_path) as connection:
        run_row = connection.execute("SELECT * FROM job_runs WHERE run_id = ?", (run_id,)).fetchone()
        source_row = None
        if run_row and run_row["source_id"] is not None:
            source_row = connection.execute("SELECT * FROM sources WHERE id = ?", (run_row["source_id"],)).fetchone()
        if source_row is None and source_path_before:
            source_row = find_source_row(connection, source_path_before)
        if source_row is None and source_path_after:
            source_row = find_source_row(connection, source_path_after)
        if source_row is None:
            source_row = upsert_source_row(
                connection,
                browser_url=None,
                source_handle=None,
                canonical_path=source_path_before,
                current_path=source_path_after,
                display_name=PurePosixPath(source_path_after or source_path_before).name,
                status=run_status,
                last_run_id=run_id,
            )
            source_id = source_row["id"]
        else:
            source_id = source_row["id"]

        if source_path_before and source_path_after and source_path_before != source_path_after:
            rewrite_paths(connection, source_path_before, source_path_after)

        connection.execute(
            """
            UPDATE job_runs
            SET source_id = ?,
                source_path_before = ?,
                source_path_after = ?,
                status = ?,
                processed = ?,
                skipped = ?,
                failed = ?,
                discovered = ?,
                manifest_local_path = ?,
                manifest_s4_key = ?,
                local_artifact_dir = ?,
                compute_profile = ?,
                selection_policy = ?,
                instance_type = ?,
                gpu_count = ?,
                architecture = ?,
                image_id = ?,
                image_family = ?,
                image_version = ?,
                selected_region = ?,
                scheduler_batch_id = ?,
                scheduler_policy = ?,
                worker_name = ?,
                runner_host = ?,
                assigned_file_kinds = ?,
                error_text = ?,
                finished_at = ?
            WHERE run_id = ?
            """,
            (
                source_id,
                source_path_before,
                source_path_after,
                run_status,
                manifest["summary"]["processed"],
                manifest["summary"]["skipped"],
                manifest["summary"]["failed"],
                manifest["summary"]["discovered"],
                str(manifest_path),
                manifest_s4_key,
                str(Path(manifest_path).parent),
                manifest.get("compute_profile"),
                manifest.get("selection_policy"),
                manifest.get("instance_type"),
                manifest.get("gpu_count"),
                manifest.get("architecture"),
                manifest.get("image_id"),
                manifest.get("image_family"),
                manifest.get("image_version"),
                manifest.get("selected_region"),
                manifest.get("scheduler_batch_id"),
                manifest.get("scheduler_policy"),
                manifest.get("worker_name"),
                manifest.get("runner_host"),
                ",".join(manifest.get("assigned_file_kinds", [])) if isinstance(manifest.get("assigned_file_kinds"), list) else manifest.get("assigned_file_kinds"),
                manifest.get("rename_error"),
                utc_now(),
                run_id,
            ),
        )
        connection.execute(
            """
            UPDATE sources
            SET current_path = ?,
                status = ?,
                last_run_id = ?,
                last_error = ?,
                updated_at = ?
            WHERE id = ?
            """,
            (
                source_path_after,
                run_status,
                run_id,
                manifest.get("rename_error"),
                utc_now(),
                source_id,
            ),
        )
        update_manifest_item_statuses(connection, manifest, run_id=run_id)
        connection.commit()

    if bridge is not None and source_path_after and str(source_path_after).startswith("/"):
        sync_source(db_path, bridge, source_path_after)

    with connect_db(db_path) as connection:
        update_manifest_item_statuses(connection, manifest, run_id=run_id)
        recompute_folder_statuses(connection, scope_root=source_path_after or source_path_before)
        connection.execute(
            "UPDATE folders SET status = ? WHERE path = ?",
            (run_status, source_path_after or source_path_before),
        )
        connection.commit()
        export_csv(connection, db_path)
        return row_to_dict(connection.execute("SELECT * FROM job_runs WHERE run_id = ?", (run_id,)).fetchone())


def mark_run_failed(
    db_path: str | Path | None,
    *,
    run_id: str,
    source_path: str,
    error_text: str,
) -> dict:
    with connect_db(db_path) as connection:
        connection.execute(
            """
            UPDATE job_runs
            SET status = ?, error_text = ?, finished_at = ?
            WHERE run_id = ?
            """,
            ("partial_failed", error_text, utc_now(), run_id),
        )
        connection.execute(
            """
            UPDATE sources
            SET status = ?, last_run_id = ?, last_error = ?, updated_at = ?
            WHERE current_path = ? OR canonical_path = ?
            """,
            ("partial_failed", run_id, error_text, utc_now(), source_path, source_path),
        )
        recompute_folder_statuses(connection, scope_root=source_path)
        connection.commit()
        export_csv(connection, db_path)
        return row_to_dict(connection.execute("SELECT * FROM job_runs WHERE run_id = ?", (run_id,)).fetchone())


def rename_folder_done(
    db_path: str | Path | None,
    bridge: MegaSshBridge | FakeMegaBridge,
    source_path: str | PurePosixPath,
) -> dict:
    source_path_before = transcribe_mega_folder.normalize_mega_path(source_path)
    source_path_after = transcribe_mega_folder.build_done_source_path(source_path_before)

    sync_source(db_path, bridge, source_path_before)
    folder = get_folder(db_path, source_path_before)
    if folder is None:
        raise RuntimeError(f"Folder is not indexed: {source_path_before}")
    if folder["pending_file_count"] > 0:
        raise RuntimeError(f"Cannot rename folder with pending files: {source_path_before}")
    if folder["failed_file_count"] > 0:
        raise RuntimeError(f"Cannot rename folder with failed files: {source_path_before}")

    rename_applied = False
    if source_path_after != source_path_before:
        bridge.rename_path(source_path_before, source_path_after)
        rename_applied = True
        with connect_db(db_path) as connection:
            rewrite_paths(connection, str(source_path_before), str(source_path_after))
            connection.execute(
                """
                UPDATE sources
                SET current_path = ?,
                    status = ?,
                    updated_at = ?
                WHERE current_path = ? OR canonical_path = ?
                """,
                (
                    str(source_path_after),
                    "transcript_done",
                    utc_now(),
                    str(source_path_before),
                    str(source_path_before),
                ),
            )
            connection.commit()

    sync_source(db_path, bridge, source_path_after)
    with connect_db(db_path) as connection:
        connection.execute(
            """
            UPDATE sources
            SET current_path = ?,
                status = ?,
                updated_at = ?
            WHERE current_path = ? OR canonical_path = ?
            """,
            (
                str(source_path_after),
                "transcript_done",
                utc_now(),
                str(source_path_after),
                str(source_path_before),
            ),
        )
        recompute_folder_statuses(connection, scope_root=source_path_after)
        connection.execute(
            "UPDATE folders SET status = ?, updated_at = ? WHERE path = ?",
            ("transcript_done", utc_now(), str(source_path_after)),
        )
        connection.commit()
        export_csv(connection, db_path)

    return {
        "source_path_before": str(source_path_before),
        "source_path_after": str(source_path_after),
        "rename_applied": rename_applied,
        "status": "transcript_done",
    }


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile")
    parser.add_argument("--db-path")
    parser.add_argument("--ssh-host")
    parser.add_argument("--ssh-user")
    parser.add_argument("--ssh-key")
    subparsers = parser.add_subparsers(dest="command", required=True)

    subparsers.add_parser("sync-account")

    resolve_parser = subparsers.add_parser("resolve-source")
    resolve_parser.add_argument("--mega-browser-folder-url", required=True)

    list_folders_parser = subparsers.add_parser("list-folders")
    list_folders_parser.add_argument("--status")
    list_folders_parser.add_argument("--created-from")
    list_folders_parser.add_argument("--created-to")
    list_folders_parser.add_argument("--sort", default="path")
    list_folders_parser.add_argument("--limit", type=int)
    list_folders_parser.add_argument("--top-level-only", action="store_true")
    list_folders_parser.add_argument("--pending-transcription-only", action="store_true")

    list_files_parser = subparsers.add_parser("list-files")
    list_files_parser.add_argument("--folder-path", required=True)
    list_files_parser.add_argument("--sort", default="created_desc")
    list_files_parser.add_argument("--limit", type=int)
    list_files_parser.add_argument("--kind")

    show_source_parser = subparsers.add_parser("show-source")
    show_source_parser.add_argument("identifier")

    search_files_parser = subparsers.add_parser("search-files")
    search_files_parser.add_argument("query")
    search_files_parser.add_argument("--kind")
    search_files_parser.add_argument("--transcript-status")

    verify_transcripts_parser = subparsers.add_parser("verify-transcripts")
    verify_transcripts_parser.add_argument("--folder-path")
    verify_transcripts_parser.add_argument("--limit", type=int)
    verify_transcripts_parser.add_argument("--output-dir", default=str(DEFAULT_LOCAL_TRANSCRIPT_DIR))
    verify_transcripts_parser.add_argument("--validate-content", action="store_true")

    sync_transcripts_parser = subparsers.add_parser("sync-transcripts")
    sync_transcripts_parser.add_argument("--folder-path")
    sync_transcripts_parser.add_argument("--limit", type=int)
    sync_transcripts_parser.add_argument("--output-dir", default=str(DEFAULT_LOCAL_TRANSCRIPT_DIR))
    sync_transcripts_parser.add_argument("--force", action="store_true")

    analyze_failures_parser = subparsers.add_parser("analyze-failures")
    analyze_failures_parser.add_argument("--run-limit", type=int, default=10)
    analyze_failures_parser.add_argument("--file-limit", type=int, default=20)

    sync_source_parser = subparsers.add_parser("sync-source")
    sync_source_parser.add_argument("--source-path", required=True)

    rename_done_parser = subparsers.add_parser("rename-folder-done")
    rename_done_parser.add_argument("--source-path", required=True)

    start_run_parser = subparsers.add_parser("start-run")
    start_run_parser.add_argument("--source-path", required=True)
    start_run_parser.add_argument("--run-id", required=True)
    start_run_parser.add_argument("--mega-browser-folder-url")
    start_run_parser.add_argument("--scheduler-batch-id")
    start_run_parser.add_argument("--scheduler-policy")
    start_run_parser.add_argument("--worker-name")
    start_run_parser.add_argument("--runner-host")
    start_run_parser.add_argument("--assigned-file-kinds")

    ingest_run_parser = subparsers.add_parser("ingest-run")
    ingest_run_parser.add_argument("--run-id", required=True)
    ingest_run_parser.add_argument("--manifest-path", required=True)
    ingest_run_parser.add_argument("--manifest-s4-key")

    fail_run_parser = subparsers.add_parser("mark-run-failed")
    fail_run_parser.add_argument("--run-id", required=True)
    fail_run_parser.add_argument("--source-path", required=True)
    fail_run_parser.add_argument("--error-text", required=True)

    kb_sync_parser = subparsers.add_parser("kb-sync")
    kb_sync_parser.add_argument("--kb-db-path", default=str(DEFAULT_KB_DB_PATH))
    kb_sync_parser.add_argument("--transcriptions-root", default=str(DEFAULT_LOCAL_TRANSCRIPT_DIR))
    kb_sync_parser.add_argument("--kb-qdrant-path", default=str(DEFAULT_KB_QDRANT_PATH))
    kb_sync_parser.add_argument("--force", action="store_true")

    kb_reindex_parser = subparsers.add_parser("kb-reindex")
    kb_reindex_parser.add_argument("--kb-db-path", default=str(DEFAULT_KB_DB_PATH))
    kb_reindex_parser.add_argument("--transcriptions-root", default=str(DEFAULT_LOCAL_TRANSCRIPT_DIR))
    kb_reindex_parser.add_argument("--kb-qdrant-path", default=str(DEFAULT_KB_QDRANT_PATH))

    kb_status_parser = subparsers.add_parser("kb-status")
    kb_status_parser.add_argument("--kb-db-path", default=str(DEFAULT_KB_DB_PATH))
    kb_status_parser.add_argument("--transcriptions-root", default=str(DEFAULT_LOCAL_TRANSCRIPT_DIR))
    kb_status_parser.add_argument("--kb-qdrant-path", default=str(DEFAULT_KB_QDRANT_PATH))

    kb_query_parser = subparsers.add_parser("kb-query")
    kb_query_parser.add_argument("query")
    kb_query_parser.add_argument("--kb-db-path", default=str(DEFAULT_KB_DB_PATH))
    kb_query_parser.add_argument("--transcriptions-root", default=str(DEFAULT_LOCAL_TRANSCRIPT_DIR))
    kb_query_parser.add_argument("--kb-qdrant-path", default=str(DEFAULT_KB_QDRANT_PATH))
    kb_query_parser.add_argument("--limit", type=int, default=5)
    kb_query_parser.add_argument("--course-name")
    kb_query_parser.add_argument("--module-path")
    kb_query_parser.add_argument("--content-type")
    kb_query_parser.add_argument("--has-timestamps", action="store_true")
    kb_query_parser.add_argument("--answer", action="store_true")

    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    db_path = args.db_path

    if args.command in {
        "sync-account",
        "resolve-source",
        "verify-transcripts",
        "sync-transcripts",
        "sync-source",
        "ingest-run",
        "rename-folder-done",
    }:
        bridge = MegaSshBridge.from_config(
            profile_path=args.profile,
            host=args.ssh_host,
            user=args.ssh_user,
            ssh_key=args.ssh_key,
        )
    else:
        bridge = None

    if args.command == "sync-account":
        result = sync_account(db_path, bridge)
        print(json.dumps(result, indent=2))
        return 0

    if args.command == "resolve-source":
        result = resolve_source(db_path, bridge, args.mega_browser_folder_url)
        print(json.dumps(result, indent=2))
        return 0 if result["status"] != "needs_source_resolution" else 1

    if args.command == "list-folders":
        print(
            json.dumps(
                list_folders(
                    db_path,
                    status=args.status,
                    created_from=args.created_from,
                    created_to=args.created_to,
                    sort=args.sort,
                    limit=args.limit,
                    top_level_only=args.top_level_only,
                    pending_transcription_only=args.pending_transcription_only,
                ),
                indent=2,
            )
        )
        return 0

    if args.command == "list-files":
        print(
            json.dumps(
                list_files(
                    db_path,
                    folder_path=args.folder_path,
                    sort=args.sort,
                    limit=args.limit,
                    kind=args.kind,
                ),
                indent=2,
            )
        )
        return 0

    if args.command == "show-source":
        source = show_source(db_path, args.identifier)
        if source is None:
            return 1
        print(json.dumps(source, indent=2))
        return 0

    if args.command == "search-files":
        result = search_files(
            db_path,
            args.query,
            kind=args.kind,
            transcript_status=args.transcript_status,
        )
        print(json.dumps(result, indent=2))
        return 0

    if args.command == "verify-transcripts":
        result = verify_transcripts(
            db_path,
            bridge,
            output_root=args.output_dir,
            folder_path=args.folder_path,
            limit=args.limit,
            validate_content=args.validate_content,
        )
        print(json.dumps(result, indent=2))
        return 0 if result["missing_on_mega"] == 0 else 1

    if args.command == "sync-transcripts":
        result = sync_transcripts(
            db_path,
            bridge,
            output_root=args.output_dir,
            folder_path=args.folder_path,
            limit=args.limit,
            force=args.force,
        )
        print(json.dumps(result, indent=2))
        return 0 if result["failed"] == 0 and result["missing_on_mega"] == 0 else 1

    if args.command == "analyze-failures":
        result = analyze_failures(
            db_path,
            run_limit=args.run_limit,
            file_limit=args.file_limit,
        )
        print(json.dumps(result, indent=2))
        return 0

    if args.command == "sync-source":
        result = sync_source(db_path, bridge, args.source_path)
        print(json.dumps(result, indent=2))
        return 0

    if args.command == "rename-folder-done":
        result = rename_folder_done(db_path, bridge, args.source_path)
        print(json.dumps(result, indent=2))
        return 0

    if args.command == "start-run":
        result = start_run(
            db_path,
            source_path=args.source_path,
            run_id=args.run_id,
            browser_url=args.mega_browser_folder_url,
            scheduler_batch_id=args.scheduler_batch_id,
            scheduler_policy=args.scheduler_policy,
            worker_name=args.worker_name,
            runner_host=args.runner_host,
            assigned_file_kinds=args.assigned_file_kinds,
        )
        print(json.dumps(result, indent=2))
        return 0

    if args.command == "ingest-run":
        result = ingest_run(
            db_path,
            bridge,
            run_id=args.run_id,
            manifest_path=args.manifest_path,
            manifest_s4_key=args.manifest_s4_key,
        )
        print(json.dumps(result, indent=2))
        return 0

    if args.command == "mark-run-failed":
        result = mark_run_failed(
            db_path,
            run_id=args.run_id,
            source_path=args.source_path,
            error_text=args.error_text,
        )
        print(json.dumps(result, indent=2))
        return 0

    if args.command == "kb-sync":
        result = kb_sync(
            kb_db_path=args.kb_db_path,
            transcriptions_root=args.transcriptions_root,
            qdrant_path=args.kb_qdrant_path,
            catalog_db_path=db_path,
            force=args.force,
        )
        print(json.dumps(result, indent=2))
        return 0

    if args.command == "kb-reindex":
        result = kb_sync(
            kb_db_path=args.kb_db_path,
            transcriptions_root=args.transcriptions_root,
            qdrant_path=args.kb_qdrant_path,
            catalog_db_path=db_path,
            force=True,
        )
        print(json.dumps(result, indent=2))
        return 0

    if args.command == "kb-status":
        result = kb_status(
            kb_db_path=args.kb_db_path,
            transcriptions_root=args.transcriptions_root,
            qdrant_path=args.kb_qdrant_path,
            catalog_db_path=db_path,
        )
        print(json.dumps(result, indent=2))
        return 0

    if args.command == "kb-query":
        result = kb_query(
            args.query,
            kb_db_path=args.kb_db_path,
            transcriptions_root=args.transcriptions_root,
            qdrant_path=args.kb_qdrant_path,
            catalog_db_path=db_path,
            limit=args.limit,
            course_name=args.course_name,
            module_path=args.module_path,
            content_type=args.content_type,
            has_timestamps=True if args.has_timestamps else None,
            synthesize=args.answer,
        )
        print(json.dumps(result, indent=2))
        return 0

    return 1


if __name__ == "__main__":
    raise SystemExit(main())
