from __future__ import annotations

import hashlib
import json
import math
import re
import sqlite3
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path, PurePosixPath
from typing import Any, Iterable

try:
    import catalog_jobs
except ModuleNotFoundError:  # pragma: no cover - import path differs between script and package usage.
    from tools import catalog_jobs

try:
    import transcribe_mega_folder
except ModuleNotFoundError:  # pragma: no cover - import path differs between script and package usage.
    from tools import transcribe_mega_folder


DEFAULT_KB_DB_PATH = Path(__file__).resolve().parents[2] / "catalog" / "transcript_knowledge_base.db"
DEFAULT_QDRANT_PATH = Path(__file__).resolve().parents[2] / "catalog" / "transcript_qdrant"
DEFAULT_TRANSCRIPT_ROOT = Path(__file__).resolve().parents[2] / "transcriptions"
DEFAULT_EMBEDDING_MODEL = "BAAI/bge-small-en-v1.5"
SCHEMA_VERSION = "2026-03-30-v1"
TOKEN_RE = re.compile(r"\w+")
PAGE_MARKER_RE = re.compile(r"^\s*---\s*Page\s+(\d+)(?:\s*\([^)]+\))?\s*---\s*$", re.IGNORECASE)
SRT_TIME_RE = re.compile(
    r"(?P<start>\d\d:\d\d:\d\d,\d\d\d)\s*-->\s*(?P<end>\d\d:\d\d:\d\d,\d\d\d)"
)


@dataclass(frozen=True)
class RepresentationSpec:
    representation_id: str
    document_id: str
    path: Path
    remote_path: str
    representation_type: str
    content_type: str
    media_kind: str
    quality_class: str
    is_primary: bool
    text: str


@dataclass(frozen=True)
class ChunkSpec:
    chunk_id: str
    document_id: str
    representation_id: str
    section_id: str
    chunk_index: int
    section_index: int
    text: str
    content_type: str
    quality_class: str
    start_sec: float | None
    end_sec: float | None
    page_start: int | None
    page_end: int | None


@dataclass(frozen=True)
class DiscoveredDocument:
    document_id: str
    canonical_remote_path: str
    relative_path: PurePosixPath
    media_path: str | None
    transcript_path: str | None
    course_name: str
    module_path: str
    lesson_title: str
    breadcrumb: str
    content_type: str
    media_kind: str
    quality_class: str
    source_id: int | None
    browser_url: str | None
    run_id: str | None
    created_at_utc: str | None
    modified_at_utc: str | None
    content_hash: str
    representations: list[RepresentationSpec]


@dataclass(frozen=True)
class VectorPoint:
    point_id: str
    vector: list[float]
    payload: dict[str, Any]


@dataclass(frozen=True)
class VectorMatch:
    point_id: str
    score: float
    payload: dict[str, Any]


def utc_now_iso() -> str:
    return datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def stable_id(*parts: str) -> str:
    digest = hashlib.sha1("::".join(parts).encode("utf-8")).hexdigest()
    return digest


def count_tokens(text: str) -> int:
    return len(TOKEN_RE.findall(text))


def normalize_whitespace(text: str) -> str:
    return re.sub(r"[ \t]+", " ", text).strip()


def format_timecode(seconds: float | None) -> str:
    if seconds is None:
        return ""
    whole = max(int(round(seconds)), 0)
    hours = whole // 3600
    minutes = (whole % 3600) // 60
    secs = whole % 60
    if hours:
        return f"{hours:02d}:{minutes:02d}:{secs:02d}"
    return f"{minutes:02d}:{secs:02d}"


def vector_enabled_for_quality(quality_class: str) -> bool:
    return quality_class == "valid_text"


def lesson_title_from_path(path: PurePosixPath) -> str:
    candidate = path.name
    suffixes = list(path.suffixes)
    for suffix in reversed(suffixes):
        if not candidate.lower().endswith(suffix.lower()):
            continue
        candidate = candidate[: -len(suffix)]
    return candidate or path.stem


def media_kind_for_suffix(path: PurePosixPath) -> str:
    suffix = path.suffix.lower()
    if suffix in {".mp4", ".m4v", ".mkv", ".mov", ".avi", ".webm", ".mpeg", ".mpg"}:
        return "video"
    if suffix in {".mp3", ".wav", ".flac", ".ogg", ".aac", ".aiff", ".m4a"}:
        return "audio"
    if suffix == ".pdf":
        return "document"
    return "text"


class TestHashEmbedder:
    model_name = "test-hash-embedder"
    dimensions = 4

    def _vector(self, text: str) -> list[float]:
        normalized = text.lower()
        return [
            float(normalized.count("growth") + normalized.count("hook")),
            float(normalized.count("offer") + normalized.count("page")),
            float(normalized.count("pdf") + normalized.count("checklist")),
            float(normalized.count("ugc") + normalized.count("resource") + normalized.count("library")),
        ]

    def embed_texts(self, texts: list[str]) -> list[list[float]]:
        return [self._vector(text) for text in texts]

    def embed_query(self, text: str) -> list[float]:
        return self._vector(text)


class SentenceTransformerEmbedder:
    def __init__(self, model_name: str = DEFAULT_EMBEDDING_MODEL):
        self.model_name = model_name
        self._model = None
        self.dimensions = 0

    def _ensure_model(self):
        if self._model is not None:
            return self._model
        from sentence_transformers import SentenceTransformer

        self._model = SentenceTransformer(self.model_name)
        self.dimensions = int(self._model.get_sentence_embedding_dimension() or 0)
        return self._model

    def embed_texts(self, texts: list[str]) -> list[list[float]]:
        model = self._ensure_model()
        vectors = model.encode(texts, normalize_embeddings=True)
        return [list(map(float, vector)) for vector in vectors]

    def embed_query(self, text: str) -> list[float]:
        model = self._ensure_model()
        vector = model.encode([text], normalize_embeddings=True)[0]
        return list(map(float, vector))


class NullVectorStore:
    available = False

    def upsert_points(self, collection_name: str, points: list[VectorPoint], vector_size: int) -> None:
        return None

    def delete_points(self, collection_name: str, point_ids: list[str]) -> None:
        return None

    def search(
        self,
        collection_name: str,
        query_vector: list[float],
        *,
        limit: int,
        filters: dict[str, Any],
    ) -> list[VectorMatch]:
        return []

    def count(self, collection_name: str) -> int:
        return 0


class InMemoryVectorStore:
    available = True

    def __init__(self):
        self._collections: dict[str, dict[str, tuple[list[float], dict[str, Any]]]] = {}

    def upsert_points(self, collection_name: str, points: list[VectorPoint], vector_size: int) -> None:
        collection = self._collections.setdefault(collection_name, {})
        for point in points:
            collection[point.point_id] = (list(point.vector), dict(point.payload))

    def delete_points(self, collection_name: str, point_ids: list[str]) -> None:
        collection = self._collections.setdefault(collection_name, {})
        for point_id in point_ids:
            collection.pop(point_id, None)

    def search(
        self,
        collection_name: str,
        query_vector: list[float],
        *,
        limit: int,
        filters: dict[str, Any],
    ) -> list[VectorMatch]:
        collection = self._collections.get(collection_name, {})
        matches: list[VectorMatch] = []
        for point_id, (vector, payload) in collection.items():
            if not payload_matches(payload, filters):
                continue
            score = cosine_similarity(query_vector, vector)
            if score <= 0:
                continue
            matches.append(VectorMatch(point_id=point_id, score=score, payload=dict(payload)))
        matches.sort(key=lambda match: (-match.score, match.point_id))
        return matches[:limit]

    def count(self, collection_name: str) -> int:
        return len(self._collections.get(collection_name, {}))


class QdrantVectorStore:
    available = True

    def __init__(self, path: str | Path):
        self.path = Path(path)
        self._client = None
        self._models = None

    def _ensure_client(self):
        if self._client is not None:
            return self._client, self._models
        from qdrant_client import QdrantClient, models

        self.path.parent.mkdir(parents=True, exist_ok=True)
        self._client = QdrantClient(path=str(self.path))
        self._models = models
        return self._client, self._models

    def _ensure_collection(self, collection_name: str, vector_size: int) -> None:
        client, models = self._ensure_client()
        existing = {item.name for item in client.get_collections().collections}
        if collection_name in existing:
            return
        client.create_collection(
            collection_name=collection_name,
            vectors_config=models.VectorParams(size=vector_size, distance=models.Distance.COSINE),
        )

    def upsert_points(self, collection_name: str, points: list[VectorPoint], vector_size: int) -> None:
        if not points:
            return
        client, models = self._ensure_client()
        self._ensure_collection(collection_name, vector_size)
        client.upsert(
            collection_name=collection_name,
            points=[
                models.PointStruct(id=point.point_id, vector=point.vector, payload=point.payload)
                for point in points
            ],
        )

    def delete_points(self, collection_name: str, point_ids: list[str]) -> None:
        if not point_ids:
            return
        client, models = self._ensure_client()
        existing = {item.name for item in client.get_collections().collections}
        if collection_name not in existing:
            return
        client.delete(
            collection_name=collection_name,
            points_selector=models.PointIdsList(points=point_ids),
        )

    def search(
        self,
        collection_name: str,
        query_vector: list[float],
        *,
        limit: int,
        filters: dict[str, Any],
    ) -> list[VectorMatch]:
        client, models = self._ensure_client()
        existing = {item.name for item in client.get_collections().collections}
        if collection_name not in existing:
            return []
        conditions = []
        for key, value in filters.items():
            if value is None:
                continue
            if key == "has_timestamps":
                conditions.append(models.FieldCondition(key=key, match=models.MatchValue(value=bool(value))))
            else:
                conditions.append(models.FieldCondition(key=key, match=models.MatchValue(value=value)))
        query_filter = models.Filter(must=conditions) if conditions else None
        matches = client.search(
            collection_name=collection_name,
            query_vector=query_vector,
            limit=limit,
            query_filter=query_filter,
        )
        return [
            VectorMatch(point_id=str(match.id), score=float(match.score), payload=dict(match.payload or {}))
            for match in matches
        ]

    def count(self, collection_name: str) -> int:
        client, _ = self._ensure_client()
        existing = {item.name for item in client.get_collections().collections}
        if collection_name not in existing:
            return 0
        return int(client.count(collection_name=collection_name, exact=True).count)


def cosine_similarity(left: list[float], right: list[float]) -> float:
    dot = sum(a * b for a, b in zip(left, right))
    left_norm = math.sqrt(sum(a * a for a in left))
    right_norm = math.sqrt(sum(b * b for b in right))
    if left_norm == 0 or right_norm == 0:
        return 0.0
    return dot / (left_norm * right_norm)


def payload_matches(payload: dict[str, Any], filters: dict[str, Any]) -> bool:
    for key, expected in filters.items():
        if expected is None:
            continue
        actual = payload.get(key)
        if key == "has_timestamps":
            if bool(actual) != bool(expected):
                return False
            continue
        if actual != expected:
            return False
    return True


class TranscriptKnowledgeBase:
    collection_name = "transcript_chunks"

    def __init__(
        self,
        *,
        db_path: str | Path = DEFAULT_KB_DB_PATH,
        transcriptions_root: str | Path = DEFAULT_TRANSCRIPT_ROOT,
        qdrant_path: str | Path = DEFAULT_QDRANT_PATH,
        catalog_db_path: str | Path | None = None,
        vector_store: Any | None = None,
        embedder: Any | None = None,
        embedding_model: str = DEFAULT_EMBEDDING_MODEL,
    ):
        self.db_path = Path(db_path)
        self.transcriptions_root = Path(transcriptions_root)
        self.qdrant_path = Path(qdrant_path)
        self.catalog_db_path = Path(catalog_db_path) if catalog_db_path is not None else catalog_jobs.DEFAULT_DB_PATH
        self.embedder = embedder
        self.embedding_model = getattr(embedder, "model_name", embedding_model)
        self.vector_store = vector_store if vector_store is not None else self._default_vector_store()
        if self.embedder is None:
            self.embedder = self._default_embedder()
            if self.embedder is not None:
                self.embedding_model = getattr(self.embedder, "model_name", embedding_model)

    def _default_vector_store(self):
        try:
            import qdrant_client  # noqa: F401

            return QdrantVectorStore(self.qdrant_path)
        except Exception:  # pragma: no cover - import availability depends on local environment.
            return NullVectorStore()

    def _default_embedder(self):
        try:
            import sentence_transformers  # noqa: F401

            return SentenceTransformerEmbedder(self.embedding_model)
        except Exception:  # pragma: no cover - heavy deps are optional in tests.
            return None

    def connect(self) -> sqlite3.Connection:
        self.db_path.parent.mkdir(parents=True, exist_ok=True)
        connection = sqlite3.connect(self.db_path)
        connection.row_factory = sqlite3.Row
        self.ensure_schema(connection)
        return connection

    def ensure_schema(self, connection: sqlite3.Connection) -> None:
        connection.executescript(
            """
            CREATE TABLE IF NOT EXISTS kb_documents (
                document_id TEXT PRIMARY KEY,
                canonical_remote_path TEXT NOT NULL,
                relative_path TEXT NOT NULL,
                folder_path TEXT NOT NULL,
                media_path TEXT,
                transcript_path TEXT,
                source_id INTEGER,
                browser_url TEXT,
                run_id TEXT,
                course_name TEXT NOT NULL,
                module_path TEXT NOT NULL,
                lesson_title TEXT NOT NULL,
                breadcrumb TEXT NOT NULL,
                content_type TEXT NOT NULL,
                media_kind TEXT NOT NULL,
                quality_class TEXT NOT NULL,
                created_at_utc TEXT,
                modified_at_utc TEXT,
                content_hash TEXT NOT NULL,
                schema_version TEXT NOT NULL,
                embedding_model TEXT NOT NULL,
                indexed_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS kb_representations (
                representation_id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                path TEXT NOT NULL,
                remote_path TEXT NOT NULL,
                representation_type TEXT NOT NULL,
                content_type TEXT NOT NULL,
                media_kind TEXT NOT NULL,
                quality_class TEXT NOT NULL,
                is_primary INTEGER NOT NULL DEFAULT 0,
                text TEXT NOT NULL,
                FOREIGN KEY(document_id) REFERENCES kb_documents(document_id)
            );

            CREATE TABLE IF NOT EXISTS kb_sections (
                section_id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                representation_id TEXT NOT NULL,
                section_index INTEGER NOT NULL,
                text TEXT NOT NULL,
                start_sec REAL,
                end_sec REAL,
                page_start INTEGER,
                page_end INTEGER,
                token_count INTEGER NOT NULL,
                FOREIGN KEY(document_id) REFERENCES kb_documents(document_id),
                FOREIGN KEY(representation_id) REFERENCES kb_representations(representation_id)
            );

            CREATE TABLE IF NOT EXISTS kb_chunks (
                chunk_id TEXT PRIMARY KEY,
                document_id TEXT NOT NULL,
                representation_id TEXT NOT NULL,
                section_id TEXT NOT NULL,
                chunk_index INTEGER NOT NULL,
                text TEXT NOT NULL,
                token_count INTEGER NOT NULL,
                content_type TEXT NOT NULL,
                quality_class TEXT NOT NULL,
                course_name TEXT NOT NULL,
                module_path TEXT NOT NULL,
                lesson_title TEXT NOT NULL,
                breadcrumb TEXT NOT NULL,
                has_timestamps INTEGER NOT NULL DEFAULT 0,
                start_sec REAL,
                end_sec REAL,
                page_start INTEGER,
                page_end INTEGER,
                vector_enabled INTEGER NOT NULL DEFAULT 0,
                embedding_model TEXT NOT NULL,
                embedding_dimensions INTEGER NOT NULL DEFAULT 0,
                indexed_at TEXT NOT NULL,
                FOREIGN KEY(document_id) REFERENCES kb_documents(document_id),
                FOREIGN KEY(representation_id) REFERENCES kb_representations(representation_id),
                FOREIGN KEY(section_id) REFERENCES kb_sections(section_id)
            );

            CREATE INDEX IF NOT EXISTS idx_kb_documents_course_name ON kb_documents(course_name);
            CREATE INDEX IF NOT EXISTS idx_kb_documents_content_hash ON kb_documents(content_hash);
            CREATE INDEX IF NOT EXISTS idx_kb_chunks_course_name ON kb_chunks(course_name);
            CREATE INDEX IF NOT EXISTS idx_kb_chunks_content_type ON kb_chunks(content_type);
            CREATE INDEX IF NOT EXISTS idx_kb_chunks_has_timestamps ON kb_chunks(has_timestamps);
            CREATE INDEX IF NOT EXISTS idx_kb_chunks_quality_class ON kb_chunks(quality_class);
            """
        )
        try:
            connection.execute(
                """
                CREATE VIRTUAL TABLE IF NOT EXISTS kb_chunks_fts
                USING fts5(
                    chunk_id UNINDEXED,
                    text,
                    lesson_title,
                    course_name,
                    module_path,
                    breadcrumb
                )
                """
            )
        except sqlite3.OperationalError:
            pass
        connection.commit()

    def discover_documents(self) -> list[DiscoveredDocument]:
        grouped: dict[str, list[dict[str, Any]]] = {}
        for path in sorted(self.transcriptions_root.rglob("*")):
            if not path.is_file():
                continue
            if path.name == ".DS_Store":
                continue
            artifact = self._artifact_info(path)
            if artifact is None:
                continue
            grouped.setdefault(artifact["document_key"], []).append(artifact)

        catalog_connection = self._catalog_connection()
        try:
            documents = [
                self._build_document(artifacts, catalog_connection)
                for _, artifacts in sorted(grouped.items())
            ]
        finally:
            if catalog_connection is not None:
                catalog_connection.close()
        return documents

    def _catalog_connection(self) -> sqlite3.Connection | None:
        if not self.catalog_db_path.exists():
            return None
        connection = sqlite3.connect(self.catalog_db_path)
        connection.row_factory = sqlite3.Row
        return connection

    def _artifact_info(self, path: Path) -> dict[str, Any] | None:
        relative_path = path.relative_to(self.transcriptions_root)
        suffix = path.suffix.lower()
        if suffix not in {".txt", ".srt", ".md"}:
            return None

        representation_type = "plain_text"
        content_type = "text_note"
        canonical_relative = relative_path
        if suffix == ".srt":
            canonical_relative = relative_path.with_suffix("")
            representation_type = "subtitle"
            content_type = "spoken_transcript"
        elif suffix == ".md":
            canonical_relative = relative_path.with_suffix("")
            representation_type = "markdown"
            content_type = "markdown"
        elif suffix == ".txt":
            base_without_txt = relative_path.with_suffix("")
            if base_without_txt.suffix.lower() in transcribe_mega_folder.SUPPORTED_MEDIA_SUFFIXES:
                canonical_relative = base_without_txt
                representation_type = "plain_text"
                content_type = "spoken_transcript"
            elif base_without_txt.suffix.lower() == ".pdf":
                canonical_relative = base_without_txt
                representation_type = "pdf_ocr"
                content_type = "pdf_ocr"
            else:
                canonical_relative = base_without_txt
                representation_type = "plain_text"
                content_type = "text_note"

        media_kind = media_kind_for_suffix(canonical_relative)
        remote_path = "/" + relative_path.as_posix()
        canonical_remote_path = "/" + canonical_relative.as_posix()
        text = path.read_text(encoding="utf-8", errors="replace")
        quality_class = self._quality_for_text(text, representation_type)
        return {
            "path": path,
            "relative_path": relative_path,
            "remote_path": remote_path,
            "canonical_relative": canonical_relative,
            "canonical_remote_path": canonical_remote_path,
            "representation_type": representation_type,
            "content_type": content_type,
            "media_kind": media_kind,
            "text": text,
            "quality_class": quality_class,
            "document_key": canonical_remote_path,
        }

    def _quality_for_text(self, text: str, representation_type: str) -> str:
        if representation_type == "subtitle":
            normalized = normalize_whitespace(text)
            if len(normalized) < 120:
                return "too_short"
            return "valid_text"
        return catalog_jobs.classify_transcript_content(text)

    def _build_document(
        self,
        artifacts: list[dict[str, Any]],
        catalog_connection: sqlite3.Connection | None,
    ) -> DiscoveredDocument:
        artifacts = sorted(
            artifacts,
            key=lambda artifact: (
                0 if artifact["representation_type"] == "subtitle" else 1,
                artifact["representation_type"],
                artifact["remote_path"],
            ),
        )
        primary_artifact = artifacts[0]
        canonical_relative = primary_artifact["canonical_relative"]
        course_name = canonical_relative.parts[0] if canonical_relative.parts else ""
        module_parts = list(canonical_relative.parts[1:-1])
        module_path = " / ".join(module_parts)
        lesson_title = lesson_title_from_path(canonical_relative)
        breadcrumb_parts = [course_name, *module_parts, lesson_title]
        breadcrumb = " / ".join(part for part in breadcrumb_parts if part)
        catalog_metadata = self._lookup_catalog_metadata(catalog_connection, artifacts, course_name)
        created_at = catalog_metadata.get("created_at_utc")
        modified_at = catalog_metadata.get("modified_at_utc")
        source_id = catalog_metadata.get("source_id")
        browser_url = catalog_metadata.get("browser_url")
        run_id = catalog_metadata.get("run_id")

        content_hash = hashlib.sha1(
            json.dumps(
                {
                    "schema": SCHEMA_VERSION,
                    "artifacts": [
                        {
                            "path": artifact["remote_path"],
                            "representation_type": artifact["representation_type"],
                            "quality_class": artifact["quality_class"],
                            "text": artifact["text"],
                        }
                        for artifact in artifacts
                    ],
                },
                sort_keys=True,
            ).encode("utf-8")
        ).hexdigest()

        document_id = stable_id(primary_artifact["canonical_remote_path"], SCHEMA_VERSION)
        representations = [
            RepresentationSpec(
                representation_id=stable_id(document_id, artifact["representation_type"], artifact["remote_path"]),
                document_id=document_id,
                path=artifact["path"],
                remote_path=artifact["remote_path"],
                representation_type=artifact["representation_type"],
                content_type=artifact["content_type"],
                media_kind=artifact["media_kind"],
                quality_class=artifact["quality_class"],
                is_primary=index == 0,
                text=artifact["text"],
            )
            for index, artifact in enumerate(artifacts)
        ]

        transcript_path = None
        media_path = None
        if primary_artifact["representation_type"] == "subtitle":
            media_path = primary_artifact["canonical_remote_path"]
            transcript_path = next(
                (artifact["remote_path"] for artifact in artifacts if artifact["representation_type"] == "plain_text"),
                primary_artifact["remote_path"],
            )
        elif primary_artifact["content_type"] == "spoken_transcript":
            media_path = primary_artifact["canonical_remote_path"]
            transcript_path = primary_artifact["remote_path"]

        return DiscoveredDocument(
            document_id=document_id,
            canonical_remote_path=primary_artifact["canonical_remote_path"],
            relative_path=canonical_relative,
            media_path=media_path,
            transcript_path=transcript_path,
            course_name=course_name,
            module_path=module_path,
            lesson_title=lesson_title,
            breadcrumb=breadcrumb,
            content_type=primary_artifact["content_type"],
            media_kind=primary_artifact["media_kind"],
            quality_class=primary_artifact["quality_class"],
            source_id=source_id,
            browser_url=browser_url,
            run_id=run_id,
            created_at_utc=created_at,
            modified_at_utc=modified_at,
            content_hash=content_hash,
            representations=representations,
        )

    def _lookup_catalog_metadata(
        self,
        connection: sqlite3.Connection | None,
        artifacts: list[dict[str, Any]],
        course_name: str,
    ) -> dict[str, Any]:
        if connection is None:
            return {}
        candidate_paths = []
        for artifact in artifacts:
            candidate_paths.append(artifact["remote_path"])
            candidate_paths.append(artifact["canonical_remote_path"])
        for path in candidate_paths:
            row = connection.execute(
                """
                SELECT path, transcript_path, created_at_utc, modified_at_utc
                FROM files
                WHERE path = ? OR transcript_path = ?
                LIMIT 1
                """,
                (path, path),
            ).fetchone()
            if row is None:
                continue
            source_row = connection.execute(
                """
                SELECT id, browser_url
                FROM sources
                WHERE current_path = ? OR canonical_path = ?
                LIMIT 1
                """,
                (f"/{course_name}", f"/{course_name}"),
            ).fetchone()
            run_row = connection.execute(
                """
                SELECT run_id
                FROM job_runs
                WHERE source_path_after = ? OR source_path_before = ?
                ORDER BY started_at DESC
                LIMIT 1
                """,
                (f"/{course_name}", f"/{course_name}"),
            ).fetchone()
            return {
                "created_at_utc": row["created_at_utc"],
                "modified_at_utc": row["modified_at_utc"],
                "source_id": source_row["id"] if source_row is not None else None,
                "browser_url": source_row["browser_url"] if source_row is not None else None,
                "run_id": run_row["run_id"] if run_row is not None else None,
            }
        return {}

    def sync(self, *, force: bool = False) -> dict[str, Any]:
        documents = self.discover_documents()
        indexed_at = utc_now_iso()
        existing_document_ids: dict[str, str] = {}
        existing_chunk_ids_by_document: dict[str, list[str]] = {}
        with self.connect() as connection:
            rows = connection.execute("SELECT document_id, content_hash FROM kb_documents").fetchall()
            existing_document_ids = {row["document_id"]: row["content_hash"] for row in rows}
            chunk_rows = connection.execute("SELECT document_id, chunk_id FROM kb_chunks").fetchall()
            for row in chunk_rows:
                existing_chunk_ids_by_document.setdefault(row["document_id"], []).append(row["chunk_id"])

        discovered_ids = {document.document_id for document in documents}
        removed_ids = [document_id for document_id in existing_document_ids if document_id not in discovered_ids]
        deleted_chunks = 0
        with self.connect() as connection:
            for document_id in removed_ids:
                deleted_chunks += self._delete_document(connection, document_id, existing_chunk_ids_by_document.get(document_id, []))
            connection.commit()

        vector_points_indexed = 0
        documents_indexed = 0
        documents_skipped = 0
        vector_points: list[VectorPoint] = []

        with self.connect() as connection:
            for document in documents:
                current_hash = existing_document_ids.get(document.document_id)
                if not force and current_hash == document.content_hash:
                    documents_skipped += 1
                    continue

                self._delete_document(connection, document.document_id, existing_chunk_ids_by_document.get(document.document_id, []))
                chunks = self._build_chunks(document)
                self._insert_document(connection, document, chunks, indexed_at)
                documents_indexed += 1

                if self.embedder is not None and getattr(self.vector_store, "available", False):
                    eligible_chunks = [chunk for chunk in chunks if vector_enabled_for_quality(chunk.quality_class)]
                    if eligible_chunks:
                        vectors = self.embedder.embed_texts([chunk.text for chunk in eligible_chunks])
                        dimensions = getattr(self.embedder, "dimensions", len(vectors[0]) if vectors else 0)
                        for chunk, vector in zip(eligible_chunks, vectors):
                            vector_points.append(
                                VectorPoint(
                                    point_id=chunk.chunk_id,
                                    vector=vector,
                                    payload={
                                        "chunk_id": chunk.chunk_id,
                                        "document_id": chunk.document_id,
                                        "course_name": document.course_name,
                                        "module_path": document.module_path,
                                        "content_type": chunk.content_type,
                                        "quality_class": chunk.quality_class,
                                        "has_timestamps": bool(chunk.start_sec is not None and chunk.end_sec is not None),
                                    },
                                )
                            )
                            connection.execute(
                                """
                                UPDATE kb_chunks
                                SET vector_enabled = 1,
                                    embedding_dimensions = ?
                                WHERE chunk_id = ?
                                """,
                                (dimensions, chunk.chunk_id),
                            )
                        vector_points_indexed += len(eligible_chunks)

            connection.commit()

        if vector_points:
            vector_size = len(vector_points[0].vector)
            self.vector_store.upsert_points(self.collection_name, vector_points, vector_size)

        return {
            "documents_discovered": len(documents),
            "documents_indexed": documents_indexed,
            "documents_skipped": documents_skipped,
            "documents_deleted": len(removed_ids),
            "vector_chunks_indexed": vector_points_indexed,
            "indexed_at": indexed_at,
            "vector_store_available": bool(getattr(self.vector_store, "available", False)),
            "embedding_model": getattr(self.embedder, "model_name", self.embedding_model) if self.embedder is not None else None,
        }

    def _delete_document(self, connection: sqlite3.Connection, document_id: str, chunk_ids: list[str]) -> int:
        connection.execute("DELETE FROM kb_chunks_fts WHERE chunk_id IN (%s)" % ",".join("?" for _ in chunk_ids), chunk_ids) if chunk_ids else None
        connection.execute("DELETE FROM kb_chunks WHERE document_id = ?", (document_id,))
        connection.execute("DELETE FROM kb_sections WHERE document_id = ?", (document_id,))
        connection.execute("DELETE FROM kb_representations WHERE document_id = ?", (document_id,))
        connection.execute("DELETE FROM kb_documents WHERE document_id = ?", (document_id,))
        if chunk_ids:
            self.vector_store.delete_points(self.collection_name, chunk_ids)
        return len(chunk_ids)

    def _insert_document(
        self,
        connection: sqlite3.Connection,
        document: DiscoveredDocument,
        chunks: list[ChunkSpec],
        indexed_at: str,
    ) -> None:
        folder_path = "/" + document.relative_path.parent.as_posix() if document.relative_path.parent != PurePosixPath(".") else "/"
        connection.execute(
            """
            INSERT INTO kb_documents (
                document_id, canonical_remote_path, relative_path, folder_path, media_path, transcript_path,
                source_id, browser_url, run_id, course_name, module_path, lesson_title, breadcrumb,
                content_type, media_kind, quality_class, created_at_utc, modified_at_utc, content_hash,
                schema_version, embedding_model, indexed_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                document.document_id,
                document.canonical_remote_path,
                document.relative_path.as_posix(),
                folder_path,
                document.media_path,
                document.transcript_path,
                document.source_id,
                document.browser_url,
                document.run_id,
                document.course_name,
                document.module_path,
                document.lesson_title,
                document.breadcrumb,
                document.content_type,
                document.media_kind,
                document.quality_class,
                document.created_at_utc,
                document.modified_at_utc,
                document.content_hash,
                SCHEMA_VERSION,
                getattr(self.embedder, "model_name", self.embedding_model) if self.embedder is not None else self.embedding_model,
                indexed_at,
            ),
        )
        for representation in document.representations:
            connection.execute(
                """
                INSERT INTO kb_representations (
                    representation_id, document_id, path, remote_path, representation_type,
                    content_type, media_kind, quality_class, is_primary, text
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                """,
                (
                    representation.representation_id,
                    representation.document_id,
                    str(representation.path),
                    representation.remote_path,
                    representation.representation_type,
                    representation.content_type,
                    representation.media_kind,
                    representation.quality_class,
                    1 if representation.is_primary else 0,
                    representation.text,
                ),
            )

        seen_sections: set[str] = set()
        for chunk in chunks:
            if chunk.section_id not in seen_sections:
                connection.execute(
                    """
                    INSERT INTO kb_sections (
                        section_id, document_id, representation_id, section_index, text,
                        start_sec, end_sec, page_start, page_end, token_count
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    """,
                    (
                        chunk.section_id,
                        chunk.document_id,
                        chunk.representation_id,
                        chunk.section_index,
                        chunk.text,
                        chunk.start_sec,
                        chunk.end_sec,
                        chunk.page_start,
                        chunk.page_end,
                        count_tokens(chunk.text),
                    ),
                )
                seen_sections.add(chunk.section_id)

            has_timestamps = 1 if chunk.start_sec is not None and chunk.end_sec is not None else 0
            connection.execute(
                """
                INSERT INTO kb_chunks (
                    chunk_id, document_id, representation_id, section_id, chunk_index, text, token_count,
                    content_type, quality_class, course_name, module_path, lesson_title, breadcrumb,
                    has_timestamps, start_sec, end_sec, page_start, page_end, vector_enabled,
                    embedding_model, embedding_dimensions, indexed_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, ?, 0, ?)
                """,
                (
                    chunk.chunk_id,
                    chunk.document_id,
                    chunk.representation_id,
                    chunk.section_id,
                    chunk.chunk_index,
                    chunk.text,
                    count_tokens(chunk.text),
                    chunk.content_type,
                    chunk.quality_class,
                    document.course_name,
                    document.module_path,
                    document.lesson_title,
                    document.breadcrumb,
                    has_timestamps,
                    chunk.start_sec,
                    chunk.end_sec,
                    chunk.page_start,
                    chunk.page_end,
                    getattr(self.embedder, "model_name", self.embedding_model) if self.embedder is not None else self.embedding_model,
                    indexed_at,
                ),
            )
            try:
                connection.execute(
                    """
                    INSERT INTO kb_chunks_fts (chunk_id, text, lesson_title, course_name, module_path, breadcrumb)
                    VALUES (?, ?, ?, ?, ?, ?)
                    """,
                    (
                        chunk.chunk_id,
                        chunk.text,
                        document.lesson_title,
                        document.course_name,
                        document.module_path,
                        document.breadcrumb,
                    ),
                )
            except sqlite3.OperationalError:
                pass

    def _build_chunks(self, document: DiscoveredDocument) -> list[ChunkSpec]:
        primary = next(representation for representation in document.representations if representation.is_primary)
        if primary.representation_type == "subtitle":
            return self._build_subtitle_chunks(document, primary)
        if primary.representation_type == "pdf_ocr":
            return self._build_pdf_chunks(document, primary)
        return self._build_text_chunks(document, primary)

    def _build_subtitle_chunks(self, document: DiscoveredDocument, representation: RepresentationSpec) -> list[ChunkSpec]:
        cues = parse_srt(representation.text)
        if not cues:
            return self._build_text_chunks(document, representation)

        chunks: list[ChunkSpec] = []
        index = 0
        start = 0
        while start < len(cues):
            end = start
            chunk_start = cues[start]["start_sec"]
            chunk_end = chunk_start
            token_count = 0
            texts = []
            while end < len(cues):
                cue = cues[end]
                cue_tokens = count_tokens(cue["text"])
                candidate_duration = cue["end_sec"] - chunk_start
                if end > start and (candidate_duration > 90 or token_count + cue_tokens > 500):
                    break
                token_count += cue_tokens
                chunk_end = cue["end_sec"]
                texts.append(cue["text"])
                end += 1
                if token_count >= 350 and candidate_duration >= 45:
                    break
            section_id = stable_id(document.document_id, "section", str(index))
            chunk_id = stable_id(document.document_id, "chunk", str(index))
            chunks.append(
                ChunkSpec(
                    chunk_id=chunk_id,
                    document_id=document.document_id,
                    representation_id=representation.representation_id,
                    section_id=section_id,
                    chunk_index=index,
                    section_index=index,
                    text=" ".join(texts),
                    content_type=representation.content_type,
                    quality_class=representation.quality_class,
                    start_sec=chunk_start,
                    end_sec=chunk_end,
                    page_start=None,
                    page_end=None,
                )
            )
            index += 1
            overlap_start = chunk_end - 10
            next_start = end
            while next_start > start and cues[next_start - 1]["start_sec"] >= overlap_start:
                next_start -= 1
            start = next_start if next_start > start else end
        return chunks

    def _build_pdf_chunks(self, document: DiscoveredDocument, representation: RepresentationSpec) -> list[ChunkSpec]:
        page_blocks = split_pdf_pages(representation.text)
        chunks: list[ChunkSpec] = []
        chunk_index = 0
        for page_number, page_text in page_blocks:
            for section_index, text in enumerate(chunk_text(page_text, min_tokens=300, max_tokens=450)):
                section_id = stable_id(document.document_id, "section", str(chunk_index))
                chunk_id = stable_id(document.document_id, "chunk", str(chunk_index))
                chunks.append(
                    ChunkSpec(
                        chunk_id=chunk_id,
                        document_id=document.document_id,
                        representation_id=representation.representation_id,
                        section_id=section_id,
                        chunk_index=chunk_index,
                        section_index=chunk_index,
                        text=text,
                        content_type=representation.content_type,
                        quality_class=representation.quality_class,
                        start_sec=None,
                        end_sec=None,
                        page_start=page_number,
                        page_end=page_number,
                    )
                )
                chunk_index += 1
        return chunks or self._build_text_chunks(document, representation)

    def _build_text_chunks(self, document: DiscoveredDocument, representation: RepresentationSpec) -> list[ChunkSpec]:
        chunks: list[ChunkSpec] = []
        for index, text in enumerate(chunk_text(representation.text, min_tokens=350, max_tokens=500)):
            section_id = stable_id(document.document_id, "section", str(index))
            chunk_id = stable_id(document.document_id, "chunk", str(index))
            chunks.append(
                ChunkSpec(
                    chunk_id=chunk_id,
                    document_id=document.document_id,
                    representation_id=representation.representation_id,
                    section_id=section_id,
                    chunk_index=index,
                    section_index=index,
                    text=text,
                    content_type=representation.content_type,
                    quality_class=representation.quality_class,
                    start_sec=None,
                    end_sec=None,
                    page_start=None,
                    page_end=None,
                )
            )
        return chunks

    def status(self) -> dict[str, Any]:
        with self.connect() as connection:
            total_documents = connection.execute("SELECT COUNT(*) FROM kb_documents").fetchone()[0]
            total_chunks = connection.execute("SELECT COUNT(*) FROM kb_chunks").fetchone()[0]
            vector_chunks = connection.execute(
                "SELECT COUNT(*) FROM kb_chunks WHERE vector_enabled = 1"
            ).fetchone()[0]
            by_type_rows = connection.execute(
                "SELECT content_type, COUNT(*) AS count FROM kb_documents GROUP BY content_type ORDER BY content_type"
            ).fetchall()
        try:
            qdrant_points = self.vector_store.count(self.collection_name)
        except Exception:
            qdrant_points = 0
        return {
            "documents": total_documents,
            "chunks": total_chunks,
            "vector_chunks": vector_chunks,
            "vector_store_available": bool(getattr(self.vector_store, "available", False)),
            "qdrant_points": qdrant_points,
            "content_type_counts": {row["content_type"]: row["count"] for row in by_type_rows},
        }

    def query(
        self,
        query_text: str,
        *,
        limit: int = 5,
        course_name: str | None = None,
        module_path: str | None = None,
        content_type: str | None = None,
        has_timestamps: bool | None = None,
        synthesize: bool = False,
    ) -> dict[str, Any]:
        filters = {
            "course_name": course_name or None,
            "module_path": module_path or None,
            "content_type": content_type or None,
            "has_timestamps": has_timestamps,
        }
        lexical = self._lexical_search(query_text, filters=filters, limit=limit * 3)
        dense = self._dense_search(query_text, filters=filters, limit=limit * 3)

        fused_scores: dict[str, float] = {}
        sources: dict[str, dict[str, Any]] = {}
        for rank, row in enumerate(lexical, start=1):
            fused_scores[row["chunk_id"]] = fused_scores.get(row["chunk_id"], 0.0) + 1.0 / (60 + rank)
            sources.setdefault(row["chunk_id"], {"lexical": row, "dense": None})
        for rank, match in enumerate(dense, start=1):
            fused_scores[match.point_id] = fused_scores.get(match.point_id, 0.0) + 1.0 / (60 + rank)
            sources.setdefault(match.point_id, {"lexical": None, "dense": match})

        with self.connect() as connection:
            rows = []
            for chunk_id, score in sorted(fused_scores.items(), key=lambda item: (-item[1], item[0])):
                row = connection.execute(
                    """
                    SELECT chunk_id, document_id, text, content_type, quality_class, course_name,
                           module_path, lesson_title, breadcrumb, start_sec, end_sec, page_start,
                           page_end, has_timestamps
                    FROM kb_chunks
                    WHERE chunk_id = ?
                    """,
                    (chunk_id,),
                ).fetchone()
                if row is None:
                    continue
                rows.append((score, dict(row)))

        deduped_results = []
        seen_documents: set[str] = set()
        for score, row in rows:
            if row["document_id"] in seen_documents:
                continue
            seen_documents.add(row["document_id"])
            result = dict(row)
            result["score"] = score
            result["snippet"] = highlight_query_snippet(row["text"], query_text)
            result["citation_label"] = build_citation_label(row)
            result["has_timestamps"] = bool(row["has_timestamps"])
            deduped_results.append(result)
            if len(deduped_results) >= limit:
                break

        answer = None
        if synthesize and query_text.strip():
            answer = self._synthesize_answer(query_text, deduped_results)

        return {
            "query": query_text,
            "results": deduped_results,
            "answer": answer,
            "filters": filters,
        }

    def _lexical_search(self, query_text: str, *, filters: dict[str, Any], limit: int) -> list[dict[str, Any]]:
        if not query_text.strip():
            return []
        clauses = []
        parameters: list[Any] = []
        if filters.get("course_name"):
            clauses.append("c.course_name = ?")
            parameters.append(filters["course_name"])
        if filters.get("module_path"):
            clauses.append("c.module_path = ?")
            parameters.append(filters["module_path"])
        if filters.get("content_type"):
            clauses.append("c.content_type = ?")
            parameters.append(filters["content_type"])
        if filters.get("has_timestamps") is not None:
            clauses.append("c.has_timestamps = ?")
            parameters.append(1 if filters["has_timestamps"] else 0)

        with self.connect() as connection:
            try:
                sql = """
                    SELECT c.chunk_id, c.text, c.lesson_title, c.course_name, c.module_path
                    FROM kb_chunks_fts AS f
                    JOIN kb_chunks AS c ON c.chunk_id = f.chunk_id
                    WHERE kb_chunks_fts MATCH ?
                """
                query_params: list[Any] = [fts_query(query_text)]
                if clauses:
                    sql += " AND " + " AND ".join(clauses)
                    query_params.extend(parameters)
                sql += " ORDER BY bm25(kb_chunks_fts), c.chunk_id LIMIT ?"
                query_params.append(limit)
                rows = connection.execute(sql, query_params).fetchall()
                return [dict(row) for row in rows]
            except sqlite3.OperationalError:
                like_value = f"%{query_text.strip()}%"
                sql = "SELECT chunk_id, text, lesson_title, course_name, module_path FROM kb_chunks c WHERE c.text LIKE ?"
                query_params = [like_value]
                if clauses:
                    sql += " AND " + " AND ".join(clauses)
                    query_params.extend(parameters)
                sql += " ORDER BY c.chunk_id LIMIT ?"
                query_params.append(limit)
                rows = connection.execute(sql, query_params).fetchall()
                return [dict(row) for row in rows]

    def _dense_search(self, query_text: str, *, filters: dict[str, Any], limit: int) -> list[VectorMatch]:
        if not query_text.strip() or self.embedder is None or not getattr(self.vector_store, "available", False):
            return []
        query_vector = self.embedder.embed_query(query_text)
        return self.vector_store.search(self.collection_name, query_vector, limit=limit, filters=filters)

    def _synthesize_answer(self, query_text: str, results: list[dict[str, Any]]) -> dict[str, Any]:
        api_key = None
        try:
            from tools.llm_judge import _load_gemini_api_key

            api_key = _load_gemini_api_key()
        except Exception:
            api_key = None
        if not api_key:
            return {
                "text": None,
                "available": False,
                "reason": "Answer synthesis unavailable: GEMINI_API_KEY is not configured.",
            }

        try:
            import google.generativeai as genai

            genai.configure(api_key=api_key)
            model = genai.GenerativeModel("gemini-2.0-flash")
            prompt = build_answer_prompt(query_text, results)
            response = model.generate_content(prompt)
            text = getattr(response, "text", None) or ""
            return {"text": text.strip() or None, "available": bool(text.strip()), "reason": None}
        except Exception as exc:
            return {
                "text": None,
                "available": False,
                "reason": f"Answer synthesis unavailable: {exc}",
            }


def build_answer_prompt(query_text: str, results: list[dict[str, Any]]) -> str:
    context_lines = []
    for index, result in enumerate(results, start=1):
        citation = result.get("citation_label") or "No citation"
        context_lines.append(
            f"[{index}] {result['breadcrumb']} ({citation})\n{result['text']}\n"
        )
    context = "\n".join(context_lines)
    return (
        "Answer the question using only the provided transcript context. "
        "Quote citations inline like [1], [2]. If the context is insufficient, say so.\n\n"
        f"Question: {query_text}\n\nContext:\n{context}"
    )


def build_citation_label(row: dict[str, Any]) -> str:
    start_sec = row.get("start_sec")
    end_sec = row.get("end_sec")
    if start_sec is not None and end_sec is not None:
        return f"{format_timecode(start_sec)}-{format_timecode(end_sec)}"
    page_start = row.get("page_start")
    page_end = row.get("page_end")
    if page_start is not None and page_end is not None:
        if page_start == page_end:
            return f"Page {page_start}"
        return f"Pages {page_start}-{page_end}"
    return "Text"


def highlight_query_snippet(text: str, query: str, radius: int = 140) -> str:
    clean_query = query.strip()
    if not clean_query:
        return normalize_whitespace(text)[: radius * 2]
    match = re.search(re.escape(clean_query), text, flags=re.IGNORECASE)
    if match is None:
        for token in re.split(r"\s+", clean_query):
            if not token:
                continue
            match = re.search(re.escape(token), text, flags=re.IGNORECASE)
            if match is not None:
                break
    if match is None:
        return normalize_whitespace(text)[: radius * 2]
    start = max(match.start() - radius, 0)
    end = min(match.end() + radius, len(text))
    snippet = text[start:end]
    return normalize_whitespace(snippet)


def fts_query(query: str) -> str:
    tokens = [token.strip().replace('"', " ") for token in re.split(r"\s+", query) if token.strip()]
    if not tokens:
        return '""'
    return " AND ".join(f'"{token}"' for token in tokens)


def split_pdf_pages(text: str) -> list[tuple[int, str]]:
    page_blocks: list[tuple[int, str]] = []
    current_page = 1
    current_lines: list[str] = []
    for raw_line in text.splitlines():
        match = PAGE_MARKER_RE.match(raw_line)
        if match is not None:
            if current_lines:
                page_blocks.append((current_page, "\n".join(current_lines).strip()))
                current_lines = []
            current_page = int(match.group(1))
            continue
        current_lines.append(raw_line)
    if current_lines:
        page_blocks.append((current_page, "\n".join(current_lines).strip()))
    return [(page, content) for page, content in page_blocks if content]


def split_text_segments(text: str) -> list[str]:
    normalized = text.replace("\r\n", "\n")
    segments = [segment.strip() for segment in re.split(r"\n\s*\n", normalized) if segment.strip()]
    if not segments:
        stripped = normalized.strip()
        return [stripped] if stripped else []
    expanded: list[str] = []
    for segment in segments:
        if count_tokens(segment) <= 220:
            expanded.append(segment)
            continue
        sentences = [part.strip() for part in re.split(r"(?<=[.!?])\s+", segment) if part.strip()]
        if len(sentences) > 1:
            expanded.extend(sentences)
        else:
            expanded.append(segment)
    return expanded


def chunk_text(text: str, *, min_tokens: int, max_tokens: int) -> list[str]:
    segments = split_text_segments(text)
    if not segments:
        return []
    chunks: list[str] = []
    index = 0
    while index < len(segments):
        parts: list[str] = []
        tokens = 0
        end = index
        while end < len(segments):
            segment = segments[end]
            segment_tokens = count_tokens(segment)
            if end > index and tokens + segment_tokens > max_tokens:
                break
            parts.append(segment)
            tokens += segment_tokens
            end += 1
            if tokens >= min_tokens:
                break
        if not parts:
            parts.append(segments[index])
            end = index + 1
        chunks.append("\n\n".join(parts).strip())
        next_index = end - 1 if end - index > 1 else end
        index = next_index if next_index > index else end
    return chunks


def parse_srt(text: str) -> list[dict[str, Any]]:
    blocks = re.split(r"\n\s*\n", text.strip())
    cues = []
    for block in blocks:
        lines = [line.strip("\ufeff") for line in block.splitlines() if line.strip()]
        if len(lines) < 2:
            continue
        time_line = lines[1] if SRT_TIME_RE.search(lines[1]) else lines[0]
        match = SRT_TIME_RE.search(time_line)
        if match is None:
            continue
        content_lines = lines[2:] if time_line == lines[1] else lines[1:]
        content = " ".join(content_lines).strip()
        if not content:
            continue
        cues.append(
            {
                "start_sec": parse_srt_time(match.group("start")),
                "end_sec": parse_srt_time(match.group("end")),
                "text": content,
            }
        )
    return cues


def parse_srt_time(value: str) -> float:
    hours, minutes, rest = value.split(":")
    seconds, millis = rest.split(",")
    return int(hours) * 3600 + int(minutes) * 60 + int(seconds) + int(millis) / 1000.0


__all__ = [
    "DEFAULT_EMBEDDING_MODEL",
    "DEFAULT_KB_DB_PATH",
    "DEFAULT_QDRANT_PATH",
    "DEFAULT_TRANSCRIPT_ROOT",
    "InMemoryVectorStore",
    "NullVectorStore",
    "QdrantVectorStore",
    "SentenceTransformerEmbedder",
    "TestHashEmbedder",
    "TranscriptKnowledgeBase",
]
