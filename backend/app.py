"""OpenAnalyst Knowledge Base API server.

Serves the /knowledge endpoint for the OpenAnalyst CLI.
Supports two backends:
  - sqlite: existing TranscriptKnowledgeBase (SQLite + Qdrant + FTS5)
  - neo4j:  Neo4j graph backend on AWS (vector + fulltext + graph traversal)

Usage:
  python app.py                           # dev mode (no auth)
  KB_BACKEND=neo4j python app.py          # Neo4j backend
  uvicorn app:app --host 0.0.0.0 --port 8420  # production
"""

from __future__ import annotations

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from config import settings
from routers import health, knowledge

app = FastAPI(
    title="OpenAnalyst Knowledge Base API",
    description="Hybrid RAG search over course transcripts with graph expansion",
    version="1.0.0",
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(health.router)
app.include_router(knowledge.router)


@app.on_event("startup")
async def startup():
    print(f"  Backend: {settings.kb_backend}")
    if settings.kb_backend == "neo4j":
        print(f"  Neo4j:   {settings.neo4j_uri}")
        from services.neo4j_backend import Neo4jKBBackend
        backend = Neo4jKBBackend()
        try:
            backend.ensure_schema()
            print("  Neo4j schema verified.")
        except Exception as exc:
            print(f"  Neo4j schema setup failed: {exc}")
    else:
        print(f"  SQLite KB: {settings.sqlite_kb_db or '(default)'}")
    auth_mode = "API key required" if settings.valid_api_keys else "dev mode (no auth)"
    print(f"  Auth:    {auth_mode}")
    print(f"  Synthesis: {settings.synthesis_provider} ({settings.synthesis_model_resolved})")


if __name__ == "__main__":
    import uvicorn

    uvicorn.run(
        "app:app",
        host=settings.host,
        port=settings.port,
        reload=True,
    )
