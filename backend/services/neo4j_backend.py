"""Neo4j graph backend — mirrors the SQLite KB schema as a knowledge graph.

Designed for AWS-hosted Neo4j. Connects when NEO4J_URI is configured.
The graph schema mirrors the existing flat document schema and adds
relationship edges for cross-course topic linking (Phase 2).
"""

from __future__ import annotations

import math
import re
from typing import Any

from config import settings
from services.base import KBBackend
from services.synthesis import synthesize_answer

TOKEN_RE = re.compile(r"\w+")


def _count_tokens(text: str) -> int:
    return len(TOKEN_RE.findall(text))


class Neo4jKBBackend(KBBackend):
    """Knowledge graph backend using Neo4j with native vector index."""

    def __init__(self) -> None:
        self._driver = None

    @property
    def backend_name(self) -> str:
        return "neo4j"

    def _ensure_driver(self):
        if self._driver is not None:
            return self._driver
        from neo4j import GraphDatabase

        self._driver = GraphDatabase.driver(
            settings.neo4j_uri,
            auth=(settings.neo4j_user, settings.neo4j_password),
        )
        return self._driver

    def _session(self):
        driver = self._ensure_driver()
        return driver.session(database=settings.neo4j_database)

    # ═══════════════════════════════════════════════════════════════════
    #  Schema setup — run once to create constraints and indexes
    # ═══════════════════════════════════════════════════════════════════

    def ensure_schema(self) -> None:
        """Create constraints, indexes, and vector index in Neo4j."""
        with self._session() as session:
            # ── Node constraints ─────────────────────────────────────
            session.run(
                "CREATE CONSTRAINT doc_id IF NOT EXISTS "
                "FOR (d:Document) REQUIRE d.document_id IS UNIQUE"
            )
            session.run(
                "CREATE CONSTRAINT chunk_id IF NOT EXISTS "
                "FOR (c:Chunk) REQUIRE c.chunk_id IS UNIQUE"
            )
            session.run(
                "CREATE CONSTRAINT section_id IF NOT EXISTS "
                "FOR (s:Section) REQUIRE s.section_id IS UNIQUE"
            )
            session.run(
                "CREATE CONSTRAINT repr_id IF NOT EXISTS "
                "FOR (r:Representation) REQUIRE r.representation_id IS UNIQUE"
            )
            session.run(
                "CREATE CONSTRAINT course_name IF NOT EXISTS "
                "FOR (co:Course) REQUIRE co.name IS UNIQUE"
            )
            session.run(
                "CREATE CONSTRAINT topic_name IF NOT EXISTS "
                "FOR (t:Topic) REQUIRE t.name IS UNIQUE"
            )

            # ── Indexes for search ───────────────────────────────────
            session.run(
                "CREATE INDEX chunk_course IF NOT EXISTS "
                "FOR (c:Chunk) ON (c.course_name)"
            )
            session.run(
                "CREATE INDEX chunk_content_type IF NOT EXISTS "
                "FOR (c:Chunk) ON (c.content_type)"
            )
            session.run(
                "CREATE INDEX chunk_quality IF NOT EXISTS "
                "FOR (c:Chunk) ON (c.quality_class)"
            )
            session.run(
                "CREATE INDEX doc_course IF NOT EXISTS "
                "FOR (d:Document) ON (d.course_name)"
            )

            # ── Full-text index on chunk text ────────────────────────
            try:
                session.run(
                    "CREATE FULLTEXT INDEX chunk_fulltext IF NOT EXISTS "
                    "FOR (c:Chunk) ON EACH [c.text, c.lesson_title, c.course_name, c.breadcrumb]"
                )
            except Exception:
                pass  # Index may already exist

            # ── Vector index for embeddings ──────────────────────────
            try:
                session.run(
                    "CREATE VECTOR INDEX chunk_embeddings IF NOT EXISTS "
                    "FOR (c:Chunk) ON (c.embedding) "
                    "OPTIONS {indexConfig: {"
                    "  `vector.dimensions`: 384,"
                    "  `vector.similarity_function`: 'cosine'"
                    "}}"
                )
            except Exception:
                pass  # Index may already exist or Neo4j version doesn't support it

    # ═══════════════════════════════════════════════════════════════════
    #  Ingest — import documents from SQLite KB into Neo4j graph
    # ═══════════════════════════════════════════════════════════════════

    def ingest_document(self, doc: dict[str, Any], representations: list[dict], chunks: list[dict]) -> None:
        """Import a single document with its representations and chunks into Neo4j."""
        with self._session() as session:
            # Create or merge Course node
            session.run(
                "MERGE (co:Course {name: $name})",
                name=doc["course_name"],
            )

            # Create Document node
            session.run(
                """
                MERGE (d:Document {document_id: $document_id})
                SET d.canonical_remote_path = $canonical_remote_path,
                    d.relative_path = $relative_path,
                    d.folder_path = $folder_path,
                    d.media_path = $media_path,
                    d.transcript_path = $transcript_path,
                    d.course_name = $course_name,
                    d.module_path = $module_path,
                    d.lesson_title = $lesson_title,
                    d.breadcrumb = $breadcrumb,
                    d.content_type = $content_type,
                    d.media_kind = $media_kind,
                    d.quality_class = $quality_class,
                    d.content_hash = $content_hash,
                    d.schema_version = $schema_version,
                    d.indexed_at = $indexed_at
                WITH d
                MATCH (co:Course {name: $course_name})
                MERGE (co)-[:HAS_DOCUMENT]->(d)
                """,
                **doc,
            )

            # Create Representation nodes
            for rep in representations:
                session.run(
                    """
                    MERGE (r:Representation {representation_id: $representation_id})
                    SET r.path = $path,
                        r.remote_path = $remote_path,
                        r.representation_type = $representation_type,
                        r.content_type = $content_type,
                        r.media_kind = $media_kind,
                        r.quality_class = $quality_class,
                        r.is_primary = $is_primary
                    WITH r
                    MATCH (d:Document {document_id: $document_id})
                    MERGE (d)-[:HAS_REPRESENTATION]->(r)
                    """,
                    **rep,
                )

            # Create Chunk nodes
            for chunk in chunks:
                session.run(
                    """
                    MERGE (c:Chunk {chunk_id: $chunk_id})
                    SET c.text = $text,
                        c.token_count = $token_count,
                        c.content_type = $content_type,
                        c.quality_class = $quality_class,
                        c.course_name = $course_name,
                        c.module_path = $module_path,
                        c.lesson_title = $lesson_title,
                        c.breadcrumb = $breadcrumb,
                        c.has_timestamps = $has_timestamps,
                        c.start_sec = $start_sec,
                        c.end_sec = $end_sec,
                        c.page_start = $page_start,
                        c.page_end = $page_end,
                        c.chunk_index = $chunk_index,
                        c.indexed_at = $indexed_at
                    WITH c
                    MATCH (d:Document {document_id: $document_id})
                    MERGE (d)-[:HAS_CHUNK]->(c)
                    """,
                    **chunk,
                )

    def set_chunk_embedding(self, chunk_id: str, embedding: list[float]) -> None:
        """Set the vector embedding on a Chunk node."""
        with self._session() as session:
            session.run(
                "MATCH (c:Chunk {chunk_id: $chunk_id}) SET c.embedding = $embedding",
                chunk_id=chunk_id,
                embedding=embedding,
            )

    # ═══════════════════════════════════════════════════════════════════
    #  Topic graph — Phase 2 (cross-course relationships)
    # ═══════════════════════════════════════════════════════════════════

    def add_topic(self, topic_name: str) -> None:
        """Create a Topic node."""
        with self._session() as session:
            session.run("MERGE (t:Topic {name: $name})", name=topic_name)

    def link_chunk_to_topic(self, chunk_id: str, topic_name: str) -> None:
        """Create a MENTIONS relationship between a Chunk and a Topic."""
        with self._session() as session:
            session.run(
                """
                MATCH (c:Chunk {chunk_id: $chunk_id})
                MERGE (t:Topic {name: $topic_name})
                MERGE (c)-[:MENTIONS]->(t)
                """,
                chunk_id=chunk_id,
                topic_name=topic_name,
            )

    def link_topics(self, topic_a: str, topic_b: str, relationship: str = "RELATES_TO") -> None:
        """Create a relationship between two Topic nodes."""
        with self._session() as session:
            session.run(
                f"""
                MERGE (a:Topic {{name: $a}})
                MERGE (b:Topic {{name: $b}})
                MERGE (a)-[:{relationship}]->(b)
                """,
                a=topic_a,
                b=topic_b,
            )

    def link_course_to_topic(self, course_name: str, topic_name: str) -> None:
        """Create a COVERS relationship between a Course and a Topic."""
        with self._session() as session:
            session.run(
                """
                MATCH (co:Course {name: $course_name})
                MERGE (t:Topic {name: $topic_name})
                MERGE (co)-[:COVERS]->(t)
                """,
                course_name=course_name,
                topic_name=topic_name,
            )

    # ═══════════════════════════════════════════════════════════════════
    #  Query — hybrid search (fulltext + vector + graph expansion)
    # ═══════════════════════════════════════════════════════════════════

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
        filters = {
            "course_name": course_name,
            "module_path": module_path,
            "content_type": content_type,
            "has_timestamps": has_timestamps,
        }

        # Phase 1: Full-text search
        lexical_results = self._fulltext_search(query_text, filters=filters, limit=limit * 3)

        # Phase 2: Vector search (if embeddings exist)
        vector_results = self._vector_search(query_text, filters=filters, limit=limit * 3)

        # Phase 3: RRF fusion
        fused = self._rrf_fuse(lexical_results, vector_results)

        # Phase 4: Graph expansion — get related chunks via topic links
        expanded = self._graph_expand(fused, limit=limit)

        # Phase 5: Build final results
        results = expanded[:limit]

        # Phase 6: Synthesize answer
        answer = None
        if synthesize and results:
            answer = synthesize_answer(query_text, results)

        return {
            "query": query_text,
            "results": results,
            "answer": answer,
            "filters": filters,
        }

    def _fulltext_search(self, query_text: str, *, filters: dict, limit: int) -> list[dict]:
        if not query_text.strip():
            return []

        where_clauses = []
        params: dict[str, Any] = {"query": query_text, "limit": limit}

        if filters.get("course_name"):
            where_clauses.append("c.course_name = $course_name")
            params["course_name"] = filters["course_name"]
        if filters.get("module_path"):
            where_clauses.append("c.module_path = $module_path")
            params["module_path"] = filters["module_path"]
        if filters.get("content_type"):
            where_clauses.append("c.content_type = $content_type")
            params["content_type"] = filters["content_type"]
        if filters.get("has_timestamps") is not None:
            where_clauses.append("c.has_timestamps = $has_timestamps")
            params["has_timestamps"] = filters["has_timestamps"]

        where = (" AND " + " AND ".join(where_clauses)) if where_clauses else ""

        cypher = f"""
            CALL db.index.fulltext.queryNodes('chunk_fulltext', $query)
            YIELD node AS c, score
            WHERE c:Chunk{where}
            RETURN c.chunk_id AS chunk_id,
                   c.text AS text,
                   c.course_name AS course_name,
                   c.module_path AS module_path,
                   c.lesson_title AS lesson_title,
                   c.breadcrumb AS breadcrumb,
                   c.content_type AS content_type,
                   c.quality_class AS quality_class,
                   c.has_timestamps AS has_timestamps,
                   c.start_sec AS start_sec,
                   c.end_sec AS end_sec,
                   c.page_start AS page_start,
                   c.page_end AS page_end,
                   score
            ORDER BY score DESC
            LIMIT $limit
        """

        with self._session() as session:
            result = session.run(cypher, **params)
            return [dict(record) for record in result]

    def _vector_search(self, query_text: str, *, filters: dict, limit: int) -> list[dict]:
        """Vector similarity search using Neo4j's native vector index."""
        if not query_text.strip():
            return []

        # Embed the query
        try:
            from sentence_transformers import SentenceTransformer

            model = SentenceTransformer(settings.embedding_model)
            query_vector = model.encode([query_text], normalize_embeddings=True)[0].tolist()
        except Exception:
            return []  # Embeddings not available

        where_clauses = []
        params: dict[str, Any] = {"query_vector": query_vector, "limit": limit}

        if filters.get("course_name"):
            where_clauses.append("c.course_name = $course_name")
            params["course_name"] = filters["course_name"]
        if filters.get("content_type"):
            where_clauses.append("c.content_type = $content_type")
            params["content_type"] = filters["content_type"]

        where = ("WHERE " + " AND ".join(where_clauses)) if where_clauses else ""

        cypher = f"""
            CALL db.index.vector.queryNodes('chunk_embeddings', $limit, $query_vector)
            YIELD node AS c, score
            {where}
            RETURN c.chunk_id AS chunk_id,
                   c.text AS text,
                   c.course_name AS course_name,
                   c.module_path AS module_path,
                   c.lesson_title AS lesson_title,
                   c.breadcrumb AS breadcrumb,
                   c.content_type AS content_type,
                   c.quality_class AS quality_class,
                   c.has_timestamps AS has_timestamps,
                   c.start_sec AS start_sec,
                   c.end_sec AS end_sec,
                   c.page_start AS page_start,
                   c.page_end AS page_end,
                   score
            ORDER BY score DESC
            LIMIT $limit
        """

        with self._session() as session:
            result = session.run(cypher, **params)
            return [dict(record) for record in result]

    def _graph_expand(self, fused_results: list[dict], *, limit: int) -> list[dict]:
        """Expand results via graph — find related chunks through Topic nodes."""
        if not fused_results:
            return []

        top_chunk_ids = [r["chunk_id"] for r in fused_results[:5]]
        seen_ids = {r["chunk_id"] for r in fused_results}

        # Find chunks connected via shared Topics (1-2 hops)
        cypher = """
            UNWIND $chunk_ids AS cid
            MATCH (c:Chunk {chunk_id: cid})-[:MENTIONS]->(t:Topic)<-[:MENTIONS]-(related:Chunk)
            WHERE NOT related.chunk_id IN $seen_ids
            RETURN DISTINCT related.chunk_id AS chunk_id,
                   related.text AS text,
                   related.course_name AS course_name,
                   related.module_path AS module_path,
                   related.lesson_title AS lesson_title,
                   related.breadcrumb AS breadcrumb,
                   related.content_type AS content_type,
                   related.quality_class AS quality_class,
                   related.has_timestamps AS has_timestamps,
                   related.start_sec AS start_sec,
                   related.end_sec AS end_sec,
                   related.page_start AS page_start,
                   related.page_end AS page_end,
                   t.name AS via_topic,
                   0.5 AS score
            LIMIT $limit
        """

        try:
            with self._session() as session:
                result = session.run(cypher, chunk_ids=top_chunk_ids, seen_ids=list(seen_ids), limit=limit)
                graph_results = [dict(record) for record in result]
        except Exception:
            graph_results = []

        # Merge: original results first, then graph-expanded results
        combined = list(fused_results)
        for gr in graph_results:
            if gr["chunk_id"] not in seen_ids:
                gr["_graph_expanded"] = True
                combined.append(gr)
                seen_ids.add(gr["chunk_id"])

        return combined

    def _rrf_fuse(self, lexical: list[dict], dense: list[dict]) -> list[dict]:
        """Reciprocal Rank Fusion to combine lexical and vector results."""
        fused_scores: dict[str, float] = {}
        sources: dict[str, dict] = {}

        for rank, row in enumerate(lexical, start=1):
            cid = row["chunk_id"]
            fused_scores[cid] = fused_scores.get(cid, 0.0) + 1.0 / (60 + rank)
            sources.setdefault(cid, row)

        for rank, row in enumerate(dense, start=1):
            cid = row["chunk_id"]
            fused_scores[cid] = fused_scores.get(cid, 0.0) + 1.0 / (60 + rank)
            sources.setdefault(cid, row)

        sorted_ids = sorted(fused_scores.keys(), key=lambda cid: (-fused_scores[cid], cid))

        results = []
        seen_docs: set[str] = set()
        for cid in sorted_ids:
            row = sources[cid]
            doc_key = f"{row.get('course_name', '')}::{row.get('lesson_title', '')}"
            if doc_key in seen_docs:
                continue
            seen_docs.add(doc_key)
            row["score"] = fused_scores[cid]
            row["snippet"] = _highlight_snippet(row.get("text", ""), query_text="")
            row["citation_label"] = _build_citation(row)
            row["document_id"] = row.get("document_id", cid)
            results.append(row)

        return results

    # ═══════════════════════════════════════════════════════════════════
    #  Status and sync
    # ═══════════════════════════════════════════════════════════════════

    def status(self) -> dict[str, Any]:
        try:
            with self._session() as session:
                doc_count = session.run("MATCH (d:Document) RETURN count(d) AS n").single()["n"]
                chunk_count = session.run("MATCH (c:Chunk) RETURN count(c) AS n").single()["n"]
                vector_count = session.run(
                    "MATCH (c:Chunk) WHERE c.embedding IS NOT NULL RETURN count(c) AS n"
                ).single()["n"]
                course_count = session.run("MATCH (co:Course) RETURN count(co) AS n").single()["n"]
                topic_count = session.run("MATCH (t:Topic) RETURN count(t) AS n").single()["n"]

                type_counts = {}
                result = session.run(
                    "MATCH (d:Document) RETURN d.content_type AS t, count(*) AS n ORDER BY t"
                )
                for record in result:
                    type_counts[record["t"]] = record["n"]

            return {
                "documents": doc_count,
                "chunks": chunk_count,
                "vector_chunks": vector_count,
                "vector_store_available": True,
                "qdrant_points": 0,
                "content_type_counts": type_counts,
                "neo4j_connected": True,
                "neo4j_node_counts": {
                    "courses": course_count,
                    "topics": topic_count,
                    "documents": doc_count,
                    "chunks": chunk_count,
                },
            }
        except Exception as exc:
            return {
                "documents": 0,
                "chunks": 0,
                "vector_chunks": 0,
                "vector_store_available": False,
                "content_type_counts": {},
                "neo4j_connected": False,
                "neo4j_node_counts": {},
                "error": str(exc),
            }

    def sync(self, *, force: bool = False) -> dict[str, Any]:
        """Sync is handled by the migration script, not live."""
        return {
            "documents_discovered": 0,
            "documents_indexed": 0,
            "documents_skipped": 0,
            "documents_deleted": 0,
            "vector_chunks_indexed": 0,
            "indexed_at": "",
            "vector_store_available": True,
            "embedding_model": settings.embedding_model,
            "note": "Use 'python migrations/sqlite_to_neo4j.py' to sync data from SQLite to Neo4j.",
        }

    def close(self) -> None:
        if self._driver is not None:
            self._driver.close()
            self._driver = None


def _highlight_snippet(text: str, query_text: str, radius: int = 140) -> str:
    clean = re.sub(r"[ \t]+", " ", text).strip()
    return clean[: radius * 2]


def _build_citation(row: dict) -> str:
    start = row.get("start_sec")
    end = row.get("end_sec")
    if start is not None and end is not None:
        return f"{_fmt_time(start)}-{_fmt_time(end)}"
    ps = row.get("page_start")
    pe = row.get("page_end")
    if ps is not None and pe is not None:
        return f"Page {ps}" if ps == pe else f"Pages {ps}-{pe}"
    return "Text"


def _fmt_time(seconds: float | None) -> str:
    if seconds is None:
        return ""
    whole = max(int(round(seconds)), 0)
    h, m, s = whole // 3600, (whole % 3600) // 60, whole % 60
    return f"{h:02d}:{m:02d}:{s:02d}" if h else f"{m:02d}:{s:02d}"
