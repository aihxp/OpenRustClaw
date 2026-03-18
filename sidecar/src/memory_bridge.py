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

    async def store_rag_chunks(
        self,
        collection_name: str,
        chunks: List[Dict[str, Any]],
    ) -> Dict[str, Any]:
        return await self._post(
            "/rag/store",
            {
                "collection_name": collection_name,
                "chunks": chunks,
            },
        )

    async def load_rag_chunks(
        self,
        collection_name: str,
        limit: int = 1000,
    ) -> List[Dict[str, Any]]:
        response = await self._post(
            "/rag/load",
            {
                "collection_name": collection_name,
                "limit": limit,
            },
        )
        chunks = response.get("chunks", [])
        if isinstance(chunks, list):
            return [item for item in chunks if isinstance(item, dict)]
        return []

    async def list_rag_collections(self, limit: int = 100) -> List[Dict[str, Any]]:
        response = await self._post(
            "/rag/list",
            {
                "limit": limit,
            },
        )
        collections = response.get("collections", [])
        if isinstance(collections, list):
            return [item for item in collections if isinstance(item, dict)]
        return []

    async def delete_rag_collection(self, collection_name: str) -> Dict[str, Any]:
        return await self._post(
            "/rag/delete",
            {
                "collection_name": collection_name,
            },
        )

    async def fetch_old_memories(
        self,
        age_days: int = 30,
        namespace: Optional[str] = None,
        user_id: Optional[str] = None,
        limit: int = 100,
    ) -> List[Dict[str, Any]]:
        payload: Dict[str, Any] = {
            "age_days": age_days,
            "limit": limit,
        }
        if namespace:
            payload["namespace"] = namespace
        if user_id:
            payload["user_id"] = user_id

        response = await self._post("/memory/maintenance/old", payload)
        memories = response.get("memories", [])
        if isinstance(memories, list):
            return [item for item in memories if isinstance(item, dict)]
        return []

    async def store_archive_entry(self, entry: Dict[str, Any]) -> Dict[str, Any]:
        payload = {
            "id": entry.get("id", ""),
            "summary": entry.get("summary", ""),
            "source_memory_ids": entry.get("source_memory_ids", []),
            "namespace": entry.get("namespace"),
            "importance": entry.get("importance"),
            "source_type": entry.get("source_type"),
        }
        return await self._post("/memory/archive/store", payload)

    async def archive_memory_ids(self, memory_ids: List[str]) -> Dict[str, Any]:
        return await self._post(
            "/memory/archive/delete",
            {
                "memory_ids": memory_ids,
            },
        )

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
