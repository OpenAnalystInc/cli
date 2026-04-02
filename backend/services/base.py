"""Abstract base for knowledge base backends."""

from __future__ import annotations

from abc import ABC, abstractmethod
from typing import Any


class KBBackend(ABC):
    """Interface that both SQLite and Neo4j backends implement."""

    @abstractmethod
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
        ...

    @abstractmethod
    def status(self) -> dict[str, Any]:
        ...

    @abstractmethod
    def sync(self, *, force: bool = False) -> dict[str, Any]:
        ...

    @property
    @abstractmethod
    def backend_name(self) -> str:
        ...
