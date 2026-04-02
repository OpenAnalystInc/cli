"""API key authentication middleware."""

from __future__ import annotations

from fastapi import HTTPException, Security
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer

from config import settings

_bearer = HTTPBearer(auto_error=False)


async def require_api_key(
    credentials: HTTPAuthorizationCredentials | None = Security(_bearer),
) -> str:
    """Validate Bearer token against configured API keys.

    If no keys are configured (dev mode), all requests are allowed.
    """
    valid_keys = settings.valid_api_keys

    # Dev mode: no keys configured → allow all
    if not valid_keys:
        return "dev-mode"

    if credentials is None:
        raise HTTPException(
            status_code=401,
            detail="Missing Authorization header. Set OPENANALYST_API_KEY in your environment.",
        )

    token = credentials.credentials
    if token not in valid_keys:
        raise HTTPException(status_code=401, detail="Invalid API key.")

    return token
