from __future__ import annotations

import argparse
import html
import json
import re
import sqlite3
from pathlib import Path, PurePosixPath
from typing import Callable
from urllib.parse import parse_qs, quote
from wsgiref.simple_server import make_server

try:
    import catalog_jobs
except ModuleNotFoundError:  # pragma: no cover - import path differs between script and package usage.
    from tools import catalog_jobs


DEFAULT_CACHE_DB_PATH = Path(__file__).resolve().parents[1] / "catalog" / "transcript_content_cache.db"


def truthy(value: str | None) -> bool:
    return value is not None and value.lower() in {"1", "true", "yes", "on"}


def first_value(query: dict[str, list[str]], key: str, default: str | None = None) -> str | None:
    values = query.get(key)
    if not values:
        return default
    return values[0]


def html_page(title: str, body: str) -> str:
    escaped_title = html.escape(title)
    return f"""<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{escaped_title}</title>
  <style>
    :root {{
      --ink: #1f2933;
      --muted: #52606d;
      --line: #d9e2ec;
      --paper: #f8fafc;
      --card: #ffffff;
      --accent: #0f766e;
      --accent-soft: #ccfbf1;
      --warning: #9a3412;
      --warning-soft: #ffedd5;
    }}
    * {{ box-sizing: border-box; }}
    body {{
      margin: 0;
      font-family: Georgia, "Times New Roman", serif;
      color: var(--ink);
      background: linear-gradient(180deg, #f5f7fa 0%, #eef2f6 100%);
    }}
    a {{ color: var(--accent); text-decoration: none; }}
    a:hover {{ text-decoration: underline; }}
    .shell {{
      max-width: 1180px;
      margin: 0 auto;
      padding: 24px;
    }}
    nav {{
      display: flex;
      gap: 16px;
      flex-wrap: wrap;
      margin-bottom: 24px;
      padding: 12px 16px;
      background: rgba(255, 255, 255, 0.82);
      border: 1px solid var(--line);
      border-radius: 14px;
      backdrop-filter: blur(12px);
    }}
    h1, h2, h3 {{ margin: 0 0 12px; }}
    h1 {{ font-size: 2rem; }}
    h2 {{ font-size: 1.3rem; }}
    .muted {{ color: var(--muted); }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 16px;
      margin: 20px 0 28px;
    }}
    .card, .panel {{
      background: var(--card);
      border: 1px solid var(--line);
      border-radius: 18px;
      box-shadow: 0 14px 40px rgba(15, 23, 42, 0.05);
    }}
    .card {{ padding: 18px; }}
    .metric {{
      font-size: 2rem;
      line-height: 1;
      margin-bottom: 8px;
    }}
    .panel {{ padding: 20px; margin-bottom: 20px; }}
    table {{
      width: 100%;
      border-collapse: collapse;
      margin-top: 12px;
      font-size: 0.96rem;
      background: #fff;
    }}
    th, td {{
      text-align: left;
      padding: 10px 12px;
      border-bottom: 1px solid var(--line);
      vertical-align: top;
    }}
    th {{
      font-size: 0.82rem;
      letter-spacing: 0.04em;
      text-transform: uppercase;
      color: var(--muted);
    }}
    code, pre {{
      font-family: "SFMono-Regular", Menlo, Monaco, monospace;
      font-size: 0.9rem;
    }}
    pre {{
      white-space: pre-wrap;
      word-break: break-word;
      background: var(--paper);
      border: 1px solid var(--line);
      border-radius: 14px;
      padding: 16px;
      overflow-x: auto;
    }}
    form {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
      gap: 12px;
      margin-top: 16px;
    }}
    input, select, button {{
      width: 100%;
      border: 1px solid var(--line);
      border-radius: 10px;
      padding: 10px 12px;
      font: inherit;
      background: #fff;
    }}
    button {{
      background: var(--accent);
      color: #fff;
      border: none;
      cursor: pointer;
    }}
    .badge {{
      display: inline-block;
      padding: 4px 8px;
      border-radius: 999px;
      background: var(--accent-soft);
      color: var(--accent);
      font-size: 0.8rem;
      margin-right: 8px;
      margin-bottom: 8px;
    }}
    .warning {{
      background: var(--warning-soft);
      color: var(--warning);
      border: 1px solid #fdba74;
      padding: 12px 14px;
      border-radius: 12px;
    }}
    .empty {{
      padding: 14px;
      border: 1px dashed var(--line);
      border-radius: 12px;
      background: var(--paper);
      color: var(--muted);
    }}
    mark {{
      background: #fef08a;
      padding: 0 2px;
    }}
  </style>
</head>
<body>
  <div class="shell">
    <nav>
      <a href="/">Dashboard</a>
      <a href="/folders">Folders</a>
      <a href="/search">Search</a>
      <a href="/knowledge">Knowledge Base</a>
      <a href="/sources">Sources</a>
      <a href="/runs">Runs</a>
    </nav>
    {body}
  </div>
</body>
</html>
"""


def render_kv_table(rows: list[tuple[str, str]]) -> str:
    if not rows:
        return '<div class="empty">No rows to show.</div>'
    rendered_rows = "\n".join(
        f"<tr><th>{html.escape(label)}</th><td>{value}</td></tr>"
        for label, value in rows
    )
    return f"<table>{rendered_rows}</table>"


def highlight_excerpt(text: str, query: str, *, radius: int = 120) -> str:
    clean_query = query.strip()
    if not clean_query:
        return html.escape(text[: radius * 2]) or ""
    match = re.search(re.escape(clean_query), text, flags=re.IGNORECASE)
    if match is None:
        tokens = [token for token in re.split(r"\s+", clean_query) if token]
        for token in tokens:
            match = re.search(re.escape(token), text, flags=re.IGNORECASE)
            if match is not None:
                break
    if match is None:
        snippet = text[: radius * 2]
        return html.escape(snippet)
    start = max(match.start() - radius, 0)
    end = min(match.end() + radius, len(text))
    snippet = text[start:end]
    escaped = html.escape(snippet)
    pattern = re.compile(re.escape(html.escape(match.group(0))), re.IGNORECASE)
    highlighted = pattern.sub(lambda found: f"<mark>{found.group(0)}</mark>", escaped, count=1)
    if start > 0:
        highlighted = "... " + highlighted
    if end < len(text):
        highlighted = highlighted + " ..."
    return highlighted


class TranscriptCache:
    def __init__(self, db_path: str | Path):
        self.db_path = Path(db_path)

    def connect(self) -> sqlite3.Connection:
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        connection = sqlite3.connect(self.db_path)
        connection.row_factory = sqlite3.Row
        self.ensure_schema(connection)
        return connection

    def ensure_schema(self, connection: sqlite3.Connection) -> None:
        connection.executescript(
            """
            CREATE TABLE IF NOT EXISTS transcript_cache (
                transcript_path TEXT PRIMARY KEY,
                source_file_path TEXT,
                content TEXT,
                fetched_at TEXT,
                fetch_error TEXT,
                error_at TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_transcript_cache_source_file_path
            ON transcript_cache(source_file_path);
            """
        )
        try:
            connection.execute(
                """
                CREATE VIRTUAL TABLE IF NOT EXISTS transcript_cache_fts
                USING fts5(transcript_path, content)
                """
            )
        except sqlite3.OperationalError:
            pass
        connection.commit()

    def _has_fts(self, connection: sqlite3.Connection) -> bool:
        row = connection.execute(
            "SELECT name FROM sqlite_master WHERE type = 'table' AND name = 'transcript_cache_fts'"
        ).fetchone()
        return row is not None

    def get_cached_entry(self, transcript_path: str | PurePosixPath) -> dict | None:
        normalized_path = str(PurePosixPath(transcript_path))
        with self.connect() as connection:
            row = connection.execute(
                "SELECT * FROM transcript_cache WHERE transcript_path = ?",
                (normalized_path,),
            ).fetchone()
            return dict(row) if row is not None else None

    def store_content(
        self,
        transcript_path: str | PurePosixPath,
        source_file_path: str | PurePosixPath | None,
        content: str,
    ) -> dict:
        normalized_path = str(PurePosixPath(transcript_path))
        normalized_source = str(PurePosixPath(source_file_path)) if source_file_path is not None else None
        fetched_at = catalog_jobs.utc_now()
        with self.connect() as connection:
            connection.execute(
                """
                INSERT INTO transcript_cache (
                    transcript_path, source_file_path, content, fetched_at, fetch_error, error_at
                ) VALUES (?, ?, ?, ?, NULL, NULL)
                ON CONFLICT(transcript_path) DO UPDATE SET
                    source_file_path = excluded.source_file_path,
                    content = excluded.content,
                    fetched_at = excluded.fetched_at,
                    fetch_error = NULL,
                    error_at = NULL
                """,
                (normalized_path, normalized_source, content, fetched_at),
            )
            if self._has_fts(connection):
                connection.execute(
                    "DELETE FROM transcript_cache_fts WHERE transcript_path = ?",
                    (normalized_path,),
                )
                connection.execute(
                    "INSERT INTO transcript_cache_fts (transcript_path, content) VALUES (?, ?)",
                    (normalized_path, content),
                )
            connection.commit()
            row = connection.execute(
                "SELECT * FROM transcript_cache WHERE transcript_path = ?",
                (normalized_path,),
            ).fetchone()
            return dict(row)

    def store_error(
        self,
        transcript_path: str | PurePosixPath,
        source_file_path: str | PurePosixPath | None,
        error_text: str,
    ) -> dict:
        normalized_path = str(PurePosixPath(transcript_path))
        normalized_source = str(PurePosixPath(source_file_path)) if source_file_path is not None else None
        error_at = catalog_jobs.utc_now()
        with self.connect() as connection:
            connection.execute(
                """
                INSERT INTO transcript_cache (
                    transcript_path, source_file_path, content, fetched_at, fetch_error, error_at
                ) VALUES (?, ?, NULL, NULL, ?, ?)
                ON CONFLICT(transcript_path) DO UPDATE SET
                    source_file_path = excluded.source_file_path,
                    fetch_error = excluded.fetch_error,
                    error_at = excluded.error_at
                """,
                (normalized_path, normalized_source, error_text, error_at),
            )
            if self._has_fts(connection):
                connection.execute(
                    "DELETE FROM transcript_cache_fts WHERE transcript_path = ?",
                    (normalized_path,),
                )
            connection.commit()
            row = connection.execute(
                "SELECT * FROM transcript_cache WHERE transcript_path = ?",
                (normalized_path,),
            ).fetchone()
            return dict(row)

    def search(self, query: str, *, limit: int = 25) -> list[dict]:
        clean_query = query.strip()
        if not clean_query:
            return []
        with self.connect() as connection:
            if self._has_fts(connection):
                try:
                    rows = connection.execute(
                        """
                        SELECT c.transcript_path, c.source_file_path, c.content
                        FROM transcript_cache_fts AS f
                        JOIN transcript_cache AS c ON c.transcript_path = f.transcript_path
                        WHERE transcript_cache_fts MATCH ?
                        ORDER BY bm25(transcript_cache_fts), c.transcript_path
                        LIMIT ?
                        """,
                        (self._fts_query(clean_query), limit),
                    ).fetchall()
                    return [dict(row) for row in rows]
                except sqlite3.OperationalError:
                    pass

            like_value = f"%{clean_query}%"
            rows = connection.execute(
                """
                SELECT transcript_path, source_file_path, content
                FROM transcript_cache
                WHERE content LIKE ?
                ORDER BY transcript_path
                LIMIT ?
                """,
                (like_value, limit),
            ).fetchall()
            return [dict(row) for row in rows]

    def _fts_query(self, query: str) -> str:
        tokens = [token.strip() for token in re.split(r"\s+", query) if token.strip()]
        if not tokens:
            return '""'
        return " AND ".join(f'"{token.replace(chr(34), chr(32))}"' for token in tokens)


class CatalogWebApp:
    def __init__(
        self,
        *,
        db_path: str | Path,
        bridge,
        cache_db_path: str | Path = DEFAULT_CACHE_DB_PATH,
        transcript_warm_batch_size: int = 25,
        knowledge_base=None,
        kb_db_path: str | Path = catalog_jobs.DEFAULT_KB_DB_PATH,
        kb_qdrant_path: str | Path = catalog_jobs.DEFAULT_KB_QDRANT_PATH,
        kb_transcriptions_root: str | Path = catalog_jobs.DEFAULT_LOCAL_TRANSCRIPT_DIR,
    ):
        self.db_path = Path(db_path)
        self.bridge = bridge
        self.cache = TranscriptCache(cache_db_path)
        self.transcript_warm_batch_size = transcript_warm_batch_size
        self._knowledge_base = knowledge_base
        self.kb_db_path = Path(kb_db_path)
        self.kb_qdrant_path = Path(kb_qdrant_path)
        self.kb_transcriptions_root = Path(kb_transcriptions_root)

    def __call__(self, environ, start_response):
        status, headers, body = self.handle_request(
            method=environ.get("REQUEST_METHOD", "GET"),
            path=environ.get("PATH_INFO", "/"),
            query=parse_qs(environ.get("QUERY_STRING", ""), keep_blank_values=True),
        )
        start_response(status, headers)
        return [body.encode("utf-8")]

    def handle_request(self, *, method: str, path: str, query: dict[str, list[str]]) -> tuple[str, list[tuple[str, str]], str]:
        if method != "GET":
            return self.respond("405 Method Not Allowed", "Method not allowed", "<div class='warning'>GET only.</div>")

        route_map: dict[str, Callable[[dict[str, list[str]]], tuple[str, str]]] = {
            "/": self.render_dashboard,
            "/folders": self.render_folders,
            "/folder": self.render_folder_detail,
            "/file": self.render_file_detail,
            "/search": self.render_search,
            "/knowledge": self.render_knowledge,
            "/sources": self.render_sources,
            "/source": self.render_source_detail,
            "/runs": self.render_runs,
            "/run": self.render_run_detail,
        }
        handler = route_map.get(path)
        if handler is None:
            return self.respond("404 Not Found", "Not Found", "<div class='warning'>Route not found.</div>")
        title, body = handler(query)
        return self.respond("200 OK", title, body)

    def respond(self, status: str, title: str, body: str) -> tuple[str, list[tuple[str, str]], str]:
        return status, [("Content-Type", "text/html; charset=utf-8")], html_page(title, body)

    def render_dashboard(self, query: dict[str, list[str]]) -> tuple[str, str]:
        summary = catalog_jobs.get_dashboard_summary(self.db_path)
        recent_folders = catalog_jobs.list_folders(self.db_path, sort="created_desc", limit=8, top_level_only=True)
        recent_runs = catalog_jobs.list_job_runs(self.db_path, limit=8)

        metrics = [
            ("Folders", summary["total_folders"]),
            ("Files", summary["total_files"]),
            ("Sources", summary["total_sources"]),
            ("Runs", summary["total_runs"]),
            ("Processed Transcripts", summary["processed_transcripts"]),
            ("Pending Transcripts", summary["pending_transcripts"]),
        ]
        cards = "\n".join(
            f"<div class='card'><div class='metric'>{value}</div><div>{html.escape(label)}</div></div>"
            for label, value in metrics
        )
        folder_statuses = "".join(
            f"<span class='badge'>{html.escape(status)}: {count}</span>"
            for status, count in summary["folder_status_counts"].items()
        )
        source_statuses = "".join(
            f"<span class='badge'>{html.escape(status)}: {count}</span>"
            for status, count in summary["source_status_counts"].items()
        )

        folder_rows = "\n".join(
            "<tr>"
            f"<td><a href='/folder?path={quote(row['path'], safe='')}'>{html.escape(row['path'])}</a></td>"
            f"<td>{html.escape(row['created_at_utc'] or '')}</td>"
            f"<td>{html.escape(row['status'])}</td>"
            f"<td>{row['media_count']}</td>"
            f"<td>{row['pending_media_count']}</td>"
            "</tr>"
            for row in recent_folders
        ) or "<tr><td colspan='5'>No folders indexed yet.</td></tr>"

        run_rows = "\n".join(
            "<tr>"
            f"<td><a href='/run?run_id={quote(row['run_id'], safe='')}'>{html.escape(row['run_id'])}</a></td>"
            f"<td>{html.escape(row['status'])}</td>"
            f"<td>{html.escape(row['source_path_after'] or row['source_path_before'] or '')}</td>"
            f"<td>{html.escape(row['started_at'])}</td>"
            "</tr>"
            for row in recent_runs
        ) or "<tr><td colspan='4'>No job runs recorded yet.</td></tr>"

        body = f"""
        <h1>Catalog Dashboard</h1>
        <p class="muted">Local read-only browser for the SQLite catalog, transcript cache, and run metadata.</p>
        <div class="grid">{cards}</div>
        <div class="panel">
          <h2>Folder Status</h2>
          {folder_statuses or '<div class="empty">No folder status rows yet.</div>'}
          <h2 style="margin-top:16px;">Source Status</h2>
          {source_statuses or '<div class="empty">No source status rows yet.</div>'}
        </div>
        <div class="panel">
          <h2>Recent Folders</h2>
          <table>
            <tr><th>Path</th><th>Added In MEGA</th><th>Status</th><th>Media</th><th>Pending</th></tr>
            {folder_rows}
          </table>
        </div>
        <div class="panel">
          <h2>Recent Runs</h2>
          <table>
            <tr><th>Run ID</th><th>Status</th><th>Source</th><th>Started</th></tr>
            {run_rows}
          </table>
        </div>
        """
        return "Catalog Dashboard", body

    def render_folders(self, query: dict[str, list[str]]) -> tuple[str, str]:
        status = first_value(query, "status")
        created_from = first_value(query, "created_from")
        created_to = first_value(query, "created_to")
        sort = first_value(query, "sort", "created_desc") or "created_desc"
        limit = first_value(query, "limit")
        top_level_only = truthy(first_value(query, "top_level_only"))
        pending_only = truthy(first_value(query, "pending_transcription_only"))

        folder_rows = catalog_jobs.list_folders(
            self.db_path,
            status=status or None,
            created_from=created_from or None,
            created_to=created_to or None,
            sort=sort,
            limit=int(limit) if limit else 50,
            top_level_only=top_level_only,
            pending_transcription_only=pending_only,
        )
        rendered_rows = "\n".join(
            "<tr>"
            f"<td><a href='/folder?path={quote(row['path'], safe='')}'>{html.escape(row['path'])}</a></td>"
            f"<td>{html.escape(row['created_at_utc'] or '')}</td>"
            f"<td>{html.escape(row['status'])}</td>"
            f"<td>{row['media_count']}</td>"
            f"<td>{row['processed_media_count']}</td>"
            f"<td>{row['pending_media_count']}</td>"
            f"<td>{row['failed_media_count']}</td>"
            "</tr>"
            for row in folder_rows
        ) or "<tr><td colspan='7'>No folders match the current filters.</td></tr>"

        body = f"""
        <h1>Folder Browser</h1>
        <p class="muted">Read-only folder structure backed by the local SQLite catalog.</p>
        <div class="panel">
          <form method="get" action="/folders">
            <input type="text" name="status" value="{html.escape(status or '')}" placeholder="status">
            <input type="text" name="created_from" value="{html.escape(created_from or '')}" placeholder="created_from">
            <input type="text" name="created_to" value="{html.escape(created_to or '')}" placeholder="created_to">
            <select name="sort">
              {self.render_select_options(sort, [('created_desc', 'created_desc'), ('created_asc', 'created_asc'), ('name', 'name'), ('path', 'path'), ('media_desc', 'media_desc'), ('processed_desc', 'processed_desc'), ('pending_desc', 'pending_desc'), ('failed_desc', 'failed_desc')])}
            </select>
            <input type="text" name="limit" value="{html.escape(limit or '50')}" placeholder="limit">
            <label><input type="checkbox" name="top_level_only" value="1" {'checked' if top_level_only else ''}> top level only</label>
            <label><input type="checkbox" name="pending_transcription_only" value="1" {'checked' if pending_only else ''}> pending only</label>
            <button type="submit">Apply Filters</button>
          </form>
          <table>
            <tr><th>Path</th><th>Added In MEGA</th><th>Status</th><th>Media</th><th>Processed</th><th>Pending</th><th>Failed</th></tr>
            {rendered_rows}
          </table>
        </div>
        """
        return "Folder Browser", body

    def render_folder_detail(self, query: dict[str, list[str]]) -> tuple[str, str]:
        folder_path = first_value(query, "path")
        if not folder_path:
            return "Folder Detail", "<div class='warning'>Missing folder path.</div>"
        folder = catalog_jobs.get_folder(self.db_path, folder_path)
        sort = first_value(query, "sort", "created_desc") or "created_desc"
        kind_filter = first_value(query, "kind") or ""
        status_filter = first_value(query, "transcript_status") or ""
        files = catalog_jobs.list_files(
            self.db_path,
            folder_path=folder_path,
            sort=sort,
            kind=kind_filter or None,
            transcript_status=status_filter or None,
        )
        file_rows = "\n".join(
            "<tr>"
            f"<td><a href='/file?path={quote(row['path'], safe='')}'>{html.escape(row['basename'])}</a></td>"
            f"<td>{html.escape(row['kind'])}</td>"
            f"<td>{html.escape(row['transcript_status'] or '')}</td>"
            f"<td>{html.escape(row['created_at_utc'] or '')}</td>"
            "</tr>"
            for row in files
        ) or "<tr><td colspan='4'>No files found for this folder.</td></tr>"
        details = []
        if folder is not None:
            details = [
                ("Path", f"<code>{html.escape(folder['path'])}</code>"),
                ("Added In MEGA", html.escape(folder["created_at_utc"] or "")),
                ("Status", html.escape(folder["status"])),
                ("Media Count", str(folder["media_count"])),
                ("Pending", str(folder["pending_media_count"])),
                ("Failed", str(folder["failed_media_count"])),
            ]
        encoded_path = quote(folder_path, safe="")
        kind_options = "".join(
            f'<option value="{k}"{" selected" if kind_filter == k else ""}>{k}</option>'
            for k in ("", "media", "document")
        )
        status_options = "".join(
            f'<option value="{s}"{" selected" if status_filter == s else ""}>{s or "all"}</option>'
            for s in ("", "pending", "processed", "failed", "skipped_existing")
        )
        sort_options = "".join(
            f'<option value="{s}"{" selected" if sort == s else ""}>{s}</option>'
            for s in ("created_desc", "modified_desc", "name")
        )
        body = f"""
        <h1>Folder Detail</h1>
        <div class="panel">{render_kv_table(details) if details else '<div class="warning">Folder not found.</div>'}</div>
        <div class="panel">
          <h2>Files</h2>
          <form method="get" action="/folder">
            <input type="hidden" name="path" value="{html.escape(folder_path)}">
            <select name="kind">{kind_options}</select>
            <select name="transcript_status">{status_options}</select>
            <select name="sort">{sort_options}</select>
            <button type="submit">Apply</button>
            <a href="/folder?path={encoded_path}">Reset</a>
          </form>
          <table>
            <tr><th>Name</th><th>Kind</th><th>Transcript Status</th><th>Created</th></tr>
            {file_rows}
          </table>
        </div>
        """
        return "Folder Detail", body

    def render_file_detail(self, query: dict[str, list[str]]) -> tuple[str, str]:
        file_path = first_value(query, "path")
        if not file_path:
            return "File Detail", "<div class='warning'>Missing file path.</div>"
        file_row = catalog_jobs.get_file(self.db_path, file_path)
        if file_row is None:
            return "File Detail", "<div class='warning'>File not found in the local catalog.</div>"

        transcript_preview = self.render_transcript_preview(file_row)
        details = [
            ("Path", f"<code>{html.escape(file_row['path'])}</code>"),
            ("Kind", html.escape(file_row["kind"])),
            ("Transcript Status", html.escape(file_row["transcript_status"] or "n/a")),
            ("Transcript Path", f"<code>{html.escape(file_row['transcript_path'] or '')}</code>"),
            ("Created", html.escape(file_row["created_at_utc"] or "")),
            ("Modified", html.escape(file_row["modified_at_utc"] or "")),
        ]
        body = f"""
        <h1>File Detail</h1>
        <div class="panel">{render_kv_table(details)}</div>
        <div class="panel">
          <h2>Transcript Preview</h2>
          {transcript_preview}
        </div>
        """
        return "File Detail", body

    def render_search(self, query: dict[str, list[str]]) -> tuple[str, str]:
        metadata_query = first_value(query, "q", "") or ""
        content_query = first_value(query, "content_q", "") or ""
        kind = first_value(query, "kind")
        transcript_status = first_value(query, "transcript_status")

        metadata_results = []
        if metadata_query.strip():
            metadata_results = catalog_jobs.search_files(
                self.db_path,
                metadata_query,
                kind=kind or None,
                transcript_status=transcript_status or None,
            )

        warm_report = {"attempted": 0, "fetched": 0, "errors": 0}
        content_results = []
        if content_query.strip():
            content_results = self.cache.search(content_query)
            if not content_results:
                warm_report = self.warm_transcript_cache(limit=self.transcript_warm_batch_size)
                content_results = self.cache.search(content_query)

        metadata_rows = "\n".join(
            "<tr>"
            f"<td><a href='/file?path={quote(row['path'], safe='')}'>{html.escape(row['path'])}</a></td>"
            f"<td>{html.escape(row['kind'])}</td>"
            f"<td>{html.escape(row['transcript_status'] or '')}</td>"
            "</tr>"
            for row in metadata_results
        ) or "<tr><td colspan='3'>No metadata matches yet.</td></tr>"

        content_rows = "\n".join(
            "<tr>"
            f"<td><a href='/file?path={quote((row.get('source_file_path') or row['transcript_path']), safe='')}'>{html.escape(row.get('source_file_path') or row['transcript_path'])}</a></td>"
            f"<td><code>{html.escape(row['transcript_path'])}</code></td>"
            f"<td>{highlight_excerpt(row.get('content') or '', content_query)}</td>"
            "</tr>"
            for row in content_results
        ) or "<tr><td colspan='3'>No transcript content matches yet.</td></tr>"

        warm_message = ""
        if content_query.strip():
            warm_message = (
                f"<p class='muted'>Transcript search warmed {warm_report['fetched']} new transcripts, "
                f"attempted {warm_report['attempted']}, errors {warm_report['errors']}. "
                "Repeated searches improve coverage as the local cache grows.</p>"
            )

        body = f"""
        <h1>Search</h1>
        <p class="muted">Search metadata immediately, and warm transcript-content search over time through the local cache.</p>
        <div class="panel">
          <form method="get" action="/search">
            <input type="text" name="q" value="{html.escape(metadata_query)}" placeholder="metadata query">
            <input type="text" name="content_q" value="{html.escape(content_query)}" placeholder="transcript text query">
            <input type="text" name="kind" value="{html.escape(kind or '')}" placeholder="kind">
            <input type="text" name="transcript_status" value="{html.escape(transcript_status or '')}" placeholder="transcript_status">
            <button type="submit">Search</button>
          </form>
        </div>
        <div class="panel">
          <h2>Metadata Matches</h2>
          <table>
            <tr><th>Path</th><th>Kind</th><th>Transcript Status</th></tr>
            {metadata_rows}
          </table>
        </div>
        <div class="panel">
          <h2>Transcript Content Matches</h2>
          {warm_message}
          <table>
            <tr><th>Source File</th><th>Transcript Path</th><th>Snippet</th></tr>
            {content_rows}
          </table>
        </div>
        """
        return "Search", body

    @property
    def knowledge_base(self):
        if self._knowledge_base is None:
            self._knowledge_base = catalog_jobs.build_knowledge_base(
                kb_db_path=self.kb_db_path,
                transcriptions_root=self.kb_transcriptions_root,
                qdrant_path=self.kb_qdrant_path,
                catalog_db_path=self.db_path,
            )
        return self._knowledge_base

    def render_knowledge(self, query: dict[str, list[str]]) -> tuple[str, str]:
        knowledge_query = first_value(query, "q", "") or ""
        course_name = first_value(query, "course_name")
        module_path = first_value(query, "module_path")
        content_type = first_value(query, "content_type")
        has_timestamps = truthy(first_value(query, "has_timestamps")) if "has_timestamps" in query else None
        answer_requested = truthy(first_value(query, "answer"))

        summary = self.knowledge_base.status()
        results = []
        answer = None
        if knowledge_query.strip():
            payload = self.knowledge_base.query(
                knowledge_query,
                limit=8,
                course_name=course_name or None,
                module_path=module_path or None,
                content_type=content_type or None,
                has_timestamps=has_timestamps,
                synthesize=answer_requested,
            )
            results = payload["results"]
            answer = payload.get("answer")

        result_rows = "\n".join(
            "<tr>"
            f"<td>{html.escape(row['lesson_title'])}</td>"
            f"<td>{html.escape(row['course_name'])}</td>"
            f"<td>{html.escape(row['content_type'])}</td>"
            f"<td>{html.escape(row['citation_label'])}</td>"
            f"<td>{html.escape(row['snippet'])}</td>"
            "</tr>"
            for row in results
        ) or "<tr><td colspan='5'>No knowledge-base matches yet.</td></tr>"

        answer_block = ""
        if answer_requested:
            if answer and answer.get("available") and answer.get("text"):
                answer_block = f"<div class='panel'><h2>Answer</h2><pre>{html.escape(answer['text'])}</pre></div>"
            else:
                reason = answer.get("reason") if answer else "Answer synthesis unavailable."
                answer_block = f"<div class='panel'><h2>Answer</h2><div class='warning'>{html.escape(reason)}</div></div>"

        body = f"""
        <h1>Knowledge Base</h1>
        <p class="muted">Hybrid transcript retrieval over the local mirror, with lexical fallback and citation-first results.</p>
        <div class="panel">
          <span class="badge">Documents: {summary['documents']}</span>
          <span class="badge">Chunks: {summary['chunks']}</span>
          <span class="badge">Vector Chunks: {summary['vector_chunks']}</span>
          <span class="badge">Qdrant Points: {summary['qdrant_points']}</span>
        </div>
        <div class="panel">
          <form method="get" action="/knowledge">
            <input type="text" name="q" value="{html.escape(knowledge_query)}" placeholder="Ask or search the transcript library">
            <input type="text" name="course_name" value="{html.escape(course_name or '')}" placeholder="course_name">
            <input type="text" name="module_path" value="{html.escape(module_path or '')}" placeholder="module_path">
            <input type="text" name="content_type" value="{html.escape(content_type or '')}" placeholder="content_type">
            <label><input type="checkbox" name="has_timestamps" value="1" {'checked' if has_timestamps else ''}> has timestamps</label>
            <label><input type="checkbox" name="answer" value="1" {'checked' if answer_requested else ''}> synthesize answer</label>
            <button type="submit">Query Knowledge Base</button>
          </form>
        </div>
        {answer_block}
        <div class="panel">
          <h2>Results</h2>
          <table>
            <tr><th>Lesson</th><th>Course</th><th>Type</th><th>Citation</th><th>Snippet</th></tr>
            {result_rows}
          </table>
        </div>
        """
        return "Knowledge Base", body

    def render_sources(self, query: dict[str, list[str]]) -> tuple[str, str]:
        rows = catalog_jobs.list_sources(self.db_path, limit=100)
        rendered_rows = "\n".join(
            "<tr>"
            f"<td><a href='/source?id={quote(row['browser_url'] or row['current_path'] or row['display_name'], safe='')}'>{html.escape(row['display_name'])}</a></td>"
            f"<td>{html.escape(row['status'])}</td>"
            f"<td>{html.escape(row['current_path'] or '')}</td>"
            f"<td>{html.escape(row['last_run_id'] or '')}</td>"
            "</tr>"
            for row in rows
        ) or "<tr><td colspan='4'>No sources tracked yet.</td></tr>"
        body = f"""
        <h1>Sources</h1>
        <div class="panel">
          <table>
            <tr><th>Name</th><th>Status</th><th>Current Path</th><th>Last Run</th></tr>
            {rendered_rows}
          </table>
        </div>
        """
        return "Sources", body

    def render_source_detail(self, query: dict[str, list[str]]) -> tuple[str, str]:
        identifier = first_value(query, "id")
        if not identifier:
            return "Source Detail", "<div class='warning'>Missing source identifier.</div>"
        row = catalog_jobs.show_source(self.db_path, identifier)
        if row is None:
            return "Source Detail", "<div class='warning'>Source not found.</div>"
        details = [
            ("Display Name", html.escape(row["display_name"])),
            ("Status", html.escape(row["status"])),
            ("Browser URL", f"<code>{html.escape(row['browser_url'] or '')}</code>"),
            ("Current Path", f"<code>{html.escape(row['current_path'] or '')}</code>"),
            ("Last Run ID", html.escape(row["last_run_id"] or "")),
            ("Last Error", html.escape(row["last_error"] or "")),
        ]
        body = f"""
        <h1>Source Detail</h1>
        <div class="panel">{render_kv_table(details)}</div>
        """
        return "Source Detail", body

    def render_runs(self, query: dict[str, list[str]]) -> tuple[str, str]:
        rows = catalog_jobs.list_job_runs(self.db_path, limit=100)
        rendered_rows = "\n".join(
            "<tr>"
            f"<td><a href='/run?run_id={quote(row['run_id'], safe='')}'>{html.escape(row['run_id'])}</a></td>"
            f"<td>{html.escape(row['status'])}</td>"
            f"<td>{html.escape(row['source_path_after'] or row['source_path_before'] or '')}</td>"
            f"<td>{html.escape(row['started_at'])}</td>"
            f"<td>{html.escape(row['finished_at'] or '')}</td>"
            "</tr>"
            for row in rows
        ) or "<tr><td colspan='5'>No runs tracked yet.</td></tr>"
        body = f"""
        <h1>Runs</h1>
        <div class="panel">
          <table>
            <tr><th>Run ID</th><th>Status</th><th>Source</th><th>Started</th><th>Finished</th></tr>
            {rendered_rows}
          </table>
        </div>
        """
        return "Runs", body

    def render_run_detail(self, query: dict[str, list[str]]) -> tuple[str, str]:
        run_id = first_value(query, "run_id")
        if not run_id:
            return "Run Detail", "<div class='warning'>Missing run id.</div>"
        run = catalog_jobs.get_job_run(self.db_path, run_id)
        if run is None:
            return "Run Detail", "<div class='warning'>Run not found.</div>"
        details = [
            ("Run ID", html.escape(run["run_id"])),
            ("Status", html.escape(run["status"])),
            ("Source Before", f"<code>{html.escape(run['source_path_before'] or '')}</code>"),
            ("Source After", f"<code>{html.escape(run['source_path_after'] or '')}</code>"),
            ("Manifest Local Path", f"<code>{html.escape(run['manifest_local_path'] or '')}</code>"),
            ("Artifact Dir", f"<code>{html.escape(run['local_artifact_dir'] or '')}</code>"),
        ]

        manifest_block = "<div class='empty'>Manifest not available yet.</div>"
        manifest_path = run.get("manifest_local_path")
        if manifest_path:
            candidate = Path(manifest_path)
            if candidate.exists():
                manifest_block = f"<pre>{html.escape(json.dumps(json.loads(candidate.read_text(encoding='utf-8')), indent=2))}</pre>"

        body = f"""
        <h1>Run Detail</h1>
        <div class="panel">{render_kv_table(details)}</div>
        <div class="panel">
          <h2>Manifest</h2>
          {manifest_block}
        </div>
        """
        return "Run Detail", body

    def render_select_options(self, selected: str, values: list[tuple[str, str]]) -> str:
        return "".join(
            f"<option value='{html.escape(value)}' {'selected' if value == selected else ''}>{html.escape(label)}</option>"
            for value, label in values
        )

    def render_transcript_preview(self, file_row: dict) -> str:
        preview_target = self.get_preview_target(file_row)
        if preview_target is None:
            return "<div class='empty'>No transcript preview is available for this file yet.</div>"

        transcript_path, source_file_path = preview_target
        cached_entry = self.ensure_transcript_available(transcript_path, source_file_path)
        if cached_entry.get("content"):
            return f"<pre>{html.escape(cached_entry['content'])}</pre>"
        if cached_entry.get("fetch_error"):
            return (
                "<div class='warning'><strong>Transcript unavailable.</strong><br>"
                f"{html.escape(cached_entry['fetch_error'])}</div>"
            )
        return "<div class='empty'>Transcript is not available yet.</div>"

    def get_preview_target(self, file_row: dict) -> tuple[str, str] | None:
        if file_row["kind"] == "media" and file_row.get("transcript_path"):
            return file_row["transcript_path"], file_row["path"]
        if file_row["extension"] == ".txt":
            return file_row["path"], file_row["path"]
        return None

    def ensure_transcript_available(self, transcript_path: str, source_file_path: str) -> dict:
        cached_entry = self.cache.get_cached_entry(transcript_path)
        if cached_entry is not None:
            if cached_entry.get("content") or cached_entry.get("fetch_error"):
                return cached_entry

        try:
            content = self.bridge.read_text_file(transcript_path)
        except Exception as exc:
            return self.cache.store_error(transcript_path, source_file_path, str(exc))
        return self.cache.store_content(transcript_path, source_file_path, content)

    def warm_transcript_cache(self, *, limit: int) -> dict[str, int]:
        candidates = catalog_jobs.list_transcript_candidates(self.db_path)
        attempted = 0
        fetched = 0
        errors = 0
        for candidate in candidates:
            if attempted >= limit:
                break
            transcript_path = candidate["transcript_path"]
            cached_entry = self.cache.get_cached_entry(transcript_path)
            if cached_entry is not None and (cached_entry.get("content") or cached_entry.get("fetch_error")):
                continue
            attempted += 1
            try:
                content = self.bridge.read_text_file(transcript_path)
            except Exception as exc:
                self.cache.store_error(transcript_path, candidate["path"], str(exc))
                errors += 1
                continue
            self.cache.store_content(transcript_path, candidate["path"], content)
            fetched += 1
        return {"attempted": attempted, "fetched": fetched, "errors": errors}


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description="Local read-only browser for the transcription catalog.")
    parser.add_argument("--db-path", type=Path, default=catalog_jobs.DEFAULT_DB_PATH)
    parser.add_argument("--cache-db-path", type=Path, default=DEFAULT_CACHE_DB_PATH)
    parser.add_argument("--kb-db-path", type=Path, default=catalog_jobs.DEFAULT_KB_DB_PATH)
    parser.add_argument("--kb-qdrant-path", type=Path, default=catalog_jobs.DEFAULT_KB_QDRANT_PATH)
    parser.add_argument("--kb-transcriptions-root", type=Path, default=catalog_jobs.DEFAULT_LOCAL_TRANSCRIPT_DIR)
    parser.add_argument("--profile")
    parser.add_argument("--ssh-host")
    parser.add_argument("--ssh-user")
    parser.add_argument("--ssh-key")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8765)
    parser.add_argument("--warm-batch-size", type=int, default=25)
    return parser


def main(argv: list[str] | None = None) -> int:
    parser = build_parser()
    args = parser.parse_args(argv)
    bridge = catalog_jobs.MegaSshBridge.from_config(
        profile_path=args.profile,
        host=args.ssh_host,
        user=args.ssh_user,
        ssh_key=args.ssh_key,
    )
    app = CatalogWebApp(
        db_path=args.db_path,
        bridge=bridge,
        cache_db_path=args.cache_db_path,
        transcript_warm_batch_size=args.warm_batch_size,
        kb_db_path=args.kb_db_path,
        kb_qdrant_path=args.kb_qdrant_path,
        kb_transcriptions_root=args.kb_transcriptions_root,
    )
    with make_server(args.host, args.port, app) as server:
        print(f"Catalog browser running on http://{args.host}:{args.port}")
        server.serve_forever()
    return 0


if __name__ == "__main__":  # pragma: no cover - exercised via manual use.
    raise SystemExit(main())
