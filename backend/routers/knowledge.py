"""Knowledge Base API endpoints."""

from __future__ import annotations

from fastapi import APIRouter, Depends
from fastapi.responses import JSONResponse

from auth import require_api_key
from config import settings
from models import (
    ChunkResult,
    KBStatusResponse,
    KnowledgeQuery,
    KnowledgeResponse,
    SyncResponse,
    SynthesisResult,
)

router = APIRouter(prefix="/v1/knowledge", tags=["knowledge"])


def _get_backend():
    """Lazy-load the configured backend."""
    if settings.kb_backend == "neo4j":
        from services.neo4j_backend import Neo4jKBBackend
        return Neo4jKBBackend()
    else:
        from services.sqlite_backend import SQLiteKBBackend
        return SQLiteKBBackend()


@router.post("/query", response_model=KnowledgeResponse)
async def query_knowledge_base(
    body: KnowledgeQuery,
    _api_key: str = Depends(require_api_key),
):
    """Search the knowledge base with hybrid search (lexical + vector + graph).

    Progressive mode: metadata scan -> auto-deep transcript search -> LLM synthesis.
    """
    backend = _get_backend()
    try:
        raw = backend.query(
            body.query,
            limit=body.max_results,
            course_name=body.course_name,
            module_path=body.module_path,
            content_type=body.content_type,
            has_timestamps=body.has_timestamps,
            synthesize=body.synthesize,
        )
    except Exception as exc:
        return JSONResponse(
            status_code=200,
            content={
                "query": body.query,
                "results": [],
                "answer": {"text": None, "available": False, "reason": f"Search error: {exc}"},
                "result_count": 0,
                "backend": backend.backend_name,
                "filters": {},
            },
        )

    results = []
    for r in raw.get("results", []):
        results.append(ChunkResult(
            chunk_id=r.get("chunk_id", ""),
            document_id=r.get("document_id", ""),
            text=r.get("text", ""),
            snippet=r.get("snippet", ""),
            score=r.get("score", 0.0),
            course_name=r.get("course_name", ""),
            module_path=r.get("module_path", ""),
            lesson_title=r.get("lesson_title", ""),
            breadcrumb=r.get("breadcrumb", ""),
            content_type=r.get("content_type", ""),
            quality_class=r.get("quality_class", ""),
            citation_label=r.get("citation_label", ""),
            has_timestamps=bool(r.get("has_timestamps", False)),
            start_sec=r.get("start_sec"),
            end_sec=r.get("end_sec"),
            page_start=r.get("page_start"),
            page_end=r.get("page_end"),
        ))

    answer_raw = raw.get("answer")
    answer = None
    if answer_raw and isinstance(answer_raw, dict):
        answer = SynthesisResult(
            text=answer_raw.get("text"),
            available=answer_raw.get("available", False),
            reason=answer_raw.get("reason"),
        )

    return KnowledgeResponse(
        query=body.query,
        results=results,
        answer=answer,
        result_count=len(results),
        backend=backend.backend_name,
        filters=raw.get("filters", {}),
    )


@router.get("/status", response_model=KBStatusResponse)
async def knowledge_base_status(
    _api_key: str = Depends(require_api_key),
):
    """Get knowledge base status — document counts, vector store status, etc."""
    backend = _get_backend()
    raw = backend.status()
    return KBStatusResponse(
        backend=backend.backend_name,
        documents=raw.get("documents", 0),
        chunks=raw.get("chunks", 0),
        vector_chunks=raw.get("vector_chunks", 0),
        vector_store_available=raw.get("vector_store_available", False),
        qdrant_points=raw.get("qdrant_points", 0),
        content_type_counts=raw.get("content_type_counts", {}),
        neo4j_connected=raw.get("neo4j_connected", False),
        neo4j_node_counts=raw.get("neo4j_node_counts", {}),
    )


@router.post("/sync", response_model=SyncResponse)
async def sync_knowledge_base(
    force: bool = False,
    _api_key: str = Depends(require_api_key),
):
    """Trigger a sync of the knowledge base (discover + index new documents)."""
    backend = _get_backend()
    raw = backend.sync(force=force)
    return SyncResponse(
        documents_discovered=raw.get("documents_discovered", 0),
        documents_indexed=raw.get("documents_indexed", 0),
        documents_skipped=raw.get("documents_skipped", 0),
        documents_deleted=raw.get("documents_deleted", 0),
        vector_chunks_indexed=raw.get("vector_chunks_indexed", 0),
        indexed_at=raw.get("indexed_at", ""),
        vector_store_available=raw.get("vector_store_available", False),
        embedding_model=raw.get("embedding_model"),
    )
