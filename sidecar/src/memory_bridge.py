"""Internal memory API bridge for sidecar workflows."""

from __future__ import annotations

import asyncio
import json
import logging
import os
import urllib.error
import urllib.request
from dataclasses import dataclass
from typing import Any, Dict, List, Optional

logger = logging.getLogger(__name__)


@dataclass
class MemoryBridge:
    """Minimal client for the Rust loopback memory API."""

    base_url: str
    token: str

    @classmethod
    def from_env(cls) -> Optional["MemoryBridge"]:
        base_url = os.getenv("OPENRUSTCLAW_INTERNAL_API_URL", "").strip()
        token = os.getenv("OPENRUSTCLAW_INTERNAL_API_TOKEN", "").strip()
        if not base_url or not token:
            return None
        return cls(base_url=base_url.rstrip("/"), token=token)

    async def search_memory(
        self,
        user_id: str,
        query: str,
        limit: int = 5,
    ) -> List[Dict[str, Any]]:
        response = await self._post(
            "/memory/search",
            {
                "user_id": user_id,
                "query": query,
                "limit": limit,
            },
        )
        memories = response.get("memories", [])
        if isinstance(memories, list):
            return [item for item in memories if isinstance(item, dict)]
        return []

    async def store_memory(
        self,
        user_id: str,
        content: str,
        category: str = "semantic",
        session_id: str = "",
        importance: float = 0.7,
    ) -> Dict[str, Any]:
        return await self._post(
            "/memory/store",
            {
                "user_id": user_id,
                "content": content,
                "category": category,
                "session_id": session_id,
                "importance": importance,
            },
        )

    async def render_core_memory(self, user_id: str) -> str:
        response = await self._post(f"/memory/core/{user_id}", {})
        content = response.get("content", "")
        return content if isinstance(content, str) else ""

    async def _post(self, path: str, payload: Dict[str, Any]) -> Dict[str, Any]:
        url = f"{self.base_url}{path}"
        body = json.dumps(payload).encode("utf-8")
        headers = {
            "Content-Type": "application/json",
            "x-openrustclaw-internal-token": self.token,
        }

        def _request() -> Dict[str, Any]:
            request = urllib.request.Request(
                url,
                data=body,
                headers=headers,
                method="POST",
            )
            with urllib.request.urlopen(request, timeout=5) as response:
                raw = response.read().decode("utf-8")
                return json.loads(raw) if raw else {}

        try:
            return await asyncio.to_thread(_request)
        except urllib.error.HTTPError as exc:
            error_body = exc.read().decode("utf-8", errors="ignore")
            raise RuntimeError(
                f"Memory bridge request failed ({exc.code}): {error_body or exc.reason}"
            ) from exc
        except urllib.error.URLError as exc:
            raise RuntimeError(f"Memory bridge unavailable: {exc.reason}") from exc

