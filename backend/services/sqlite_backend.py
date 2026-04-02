"""SQLite + Qdrant backend — wraps the existing TranscriptKnowledgeBase pipeline."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

from services.base import KBBackend
from config import settings


class SQLiteKBBackend(KBBackend):
    """Delegates to the existing TranscriptKnowledgeBase from the transcription pipeline."""

    def __init__(self) -> None:
        self._kb = None

    @property
    def backend_name(self) -> str:
        return "sqlite"

    def _ensure_kb(self):
        if self._kb is not None:
            return self._kb

        # Import the existing pipeline code
        # Adjust sys.path to find the transcript_kb module
        pipeline_root = Path(settings.sqlite_transcriptions_root).parent if settings.sqlite_transcriptions_root else None
        tools_dir = pipeline_root / "tools" if pipeline_root else None

        if tools_dir and tools_dir.exists() and str(tools_dir) not in sys.path:
            sys.path.insert(0, str(tools_dir))
            sys.path.insert(0, str(pipeline_root))

        try:
            from transcript_kb import TranscriptKnowledgeBase

            kwargs: dict[str, Any] = {}
            if settings.sqlite_kb_db:
                kwargs["db_path"] = settings.sqlite_kb_db
            if settings.sqlite_transcriptions_root:
                kwargs["transcriptions_root"] = settings.sqlite_transcriptions_root
            if settings.sqlite_qdrant_path:
                kwargs["qdrant_path"] = settings.sqlite_qdrant_path
            if settings.sqlite_catalog_db:
                kwargs["catalog_db_path"] = settings.sqlite_catalog_db
            if settings.embedding_model:
                kwargs["embedding_model"] = settings.embedding_model

            self._kb = TranscriptKnowledgeBase(**kwargs)
        except ImportError:
            # Pipeline code not available — create a minimal stub
            self._kb = _StubKB()

        return self._kb

    def query(
        self,
        query_text: str,
        *,
        limit: int = 10,
        course_name: str | None = None,
        module_path: str | None = None,
        content_type: str | None = None,
        has_timestamps: bool | None = None,
        synthesize: bool = False,
    ) -> dict[str, Any]:
        kb = self._ensure_kb()
        return kb.query(
            query_text,
            limit=limit,
            course_name=course_name,
            module_path=module_path,
            content_type=content_type,
            has_timestamps=has_timestamps,
            synthesize=synthesize,
        )

    def status(self) -> dict[str, Any]:
        kb = self._ensure_kb()
        return kb.status()

    def sync(self, *, force: bool = False) -> dict[str, Any]:
        kb = self._ensure_kb()
        return kb.sync(force=force)


class _StubKB:
    """Fallback when pipeline code is not importable."""

    def query(self, query_text, **kwargs):
        return {
            "query": query_text,
            "results": [],
            "answer": {"text": None, "available": False, "reason": "SQLite backend not configured. Set SQLITE_TRANSCRIPTIONS_ROOT."},
            "filters": {},
        }

    def status(self):
        return {
            "documents": 0,
            "chunks": 0,
            "vector_chunks": 0,
            "vector_store_available": False,
            "qdrant_points": 0,
            "content_type_counts": {},
        }

    def sync(self, *, force=False):
        return {
            "documents_discovered": 0,
            "documents_indexed": 0,
            "documents_skipped": 0,
            "documents_deleted": 0,
            "vector_chunks_indexed": 0,
            "indexed_at": "",
            "vector_store_available": False,
            "embedding_model": None,
        }
