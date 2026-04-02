"""Pydantic request/response models for the Knowledge Base API."""

from __future__ import annotations

from pydantic import BaseModel, Field


# ── Request models ───────────────────────────────────────────────────────


class KnowledgeQuery(BaseModel):
    query: str = Field(..., min_length=1, description="The search query text")
    mode: str = Field("progressive", description="Search mode: progressive | lexical | dense")
    max_results: int = Field(10, ge=1, le=50, description="Maximum results to return")
    synthesize: bool = Field(True, description="Whether to synthesize an LLM answer from results")

    # Optional filters
    course_name: str | None = Field(None, description="Filter by course name")
    module_path: str | None = Field(None, description="Filter by module path")
    content_type: str | None = Field(None, description="Filter by content type: spoken_transcript | pdf_ocr | text_note")
    has_timestamps: bool | None = Field(None, description="Filter for time-aligned content only")


# ── Response models ──────────────────────────────────────────────────────


class ChunkResult(BaseModel):
    chunk_id: str
    document_id: str
    text: str
    snippet: str
    score: float
    course_name: str
    module_path: str
    lesson_title: str
    breadcrumb: str
    content_type: str
    quality_class: str
    citation_label: str
    has_timestamps: bool
    start_sec: float | None = None
    end_sec: float | None = None
    page_start: int | None = None
    page_end: int | None = None


class SynthesisResult(BaseModel):
    text: str | None
    available: bool
    reason: str | None = None


class KnowledgeResponse(BaseModel):
    query: str
    results: list[ChunkResult]
    answer: SynthesisResult | None = None
    result_count: int
    backend: str
    filters: dict


class KBStatusResponse(BaseModel):
    backend: str
    documents: int
    chunks: int
    vector_chunks: int
    vector_store_available: bool
    qdrant_points: int = 0
    content_type_counts: dict[str, int] = {}
    neo4j_connected: bool = False
    neo4j_node_counts: dict[str, int] = {}


class SyncResponse(BaseModel):
    documents_discovered: int
    documents_indexed: int
    documents_skipped: int
    documents_deleted: int
    vector_chunks_indexed: int
    indexed_at: str
    vector_store_available: bool
    embedding_model: str | None = None


class HealthResponse(BaseModel):
    status: str
    backend: str
    version: str
