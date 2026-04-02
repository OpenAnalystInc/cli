"""Health check endpoint."""

from __future__ import annotations

from fastapi import APIRouter

from config import settings
from models import HealthResponse

router = APIRouter(tags=["health"])


@router.get("/v1/health", response_model=HealthResponse)
async def health():
    return HealthResponse(
        status="ok",
        backend=settings.kb_backend,
        version="1.0.0",
    )
