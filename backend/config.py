"""Configuration loaded from environment variables."""

from __future__ import annotations

from pathlib import Path
from pydantic_settings import BaseSettings


class Settings(BaseSettings):
    # ── Server ───────────────────────────────────────────────────────────
    host: str = "0.0.0.0"
    port: int = 8420

    # ── Auth ─────────────────────────────────────────────────────────────
    # Comma-separated list of valid API keys (set OPENANALYST_API_KEYS)
    openanalyst_api_keys: str = ""

    # ── Backend selector ─────────────────────────────────────────────────
    # "sqlite" uses the existing TranscriptKnowledgeBase pipeline
    # "neo4j"  uses the Neo4j graph backend on AWS
    kb_backend: str = "sqlite"

    # ── SQLite backend ───────────────────────────────────────────────────
    sqlite_kb_db: str = ""
    sqlite_catalog_db: str = ""
    sqlite_qdrant_path: str = ""
    sqlite_transcriptions_root: str = ""
    embedding_model: str = "BAAI/bge-small-en-v1.5"

    # ── Neo4j backend ────────────────────────────────────────────────────
    neo4j_uri: str = "bolt://localhost:7687"
    neo4j_user: str = "neo4j"
    neo4j_password: str = ""
    neo4j_database: str = "neo4j"

    # ── LLM synthesis ────────────────────────────────────────────────────
    # Which provider to use for answer synthesis: gemini | openai | anthropic
    synthesis_provider: str = "gemini"
    gemini_api_key: str = ""
    openai_api_key: str = ""
    anthropic_api_key: str = ""
    synthesis_model: str = ""  # auto-selects per provider if empty

    # ── Search defaults ──────────────────────────────────────────────────
    default_result_limit: int = 10
    max_result_limit: int = 50

    model_config = {"env_prefix": "", "env_file": ".env", "extra": "ignore"}

    @property
    def valid_api_keys(self) -> set[str]:
        if not self.openanalyst_api_keys.strip():
            return set()
        return {k.strip() for k in self.openanalyst_api_keys.split(",") if k.strip()}

    @property
    def synthesis_model_resolved(self) -> str:
        if self.synthesis_model:
            return self.synthesis_model
        return {
            "gemini": "gemini-2.0-flash",
            "openai": "gpt-4o-mini",
            "anthropic": "claude-sonnet-4-20250514",
        }.get(self.synthesis_provider, "gemini-2.0-flash")


settings = Settings()
