"""Memory consolidation and maintenance workflow."""

import json
import logging
from datetime import datetime, timedelta
from typing import Annotated, Any, Dict, List, Literal, Optional, Sequence, TypedDict

from langchain_core.messages import AIMessage, BaseMessage, HumanMessage, SystemMessage
from langchain_core.runnables import RunnableConfig
from langgraph.graph import END, START, StateGraph
from langgraph.graph.message import add_messages

from ..memory_bridge import MemoryBridge

logger = logging.getLogger(__name__)


class MemoryMaintenanceState(TypedDict):
    """State for the memory maintenance graph."""

    messages: Annotated[Sequence[BaseMessage], add_messages]
    old_memories: List[Dict[str, Any]]
    summaries: List[Dict[str, Any]]
    archived_count: int
    consolidated_count: int
    archive_entries: List[Dict[str, Any]]
    archived_memory_ids: List[str]
    errors: List[str]
    status: Literal["pending", "running", "completed", "error"]


def create_default_state() -> MemoryMaintenanceState:
    """Create default initial state."""
    return {
        "messages": [],
        "old_memories": [],
        "summaries": [],
        "archived_count": 0,
        "consolidated_count": 0,
        "archive_entries": [],
        "archived_memory_ids": [],
        "errors": [],
        "status": "pending",
    }


def _load_configurable_payload(
    config: Optional[RunnableConfig],
    key: str,
) -> Optional[Any]:
    """Extract structured workflow metadata from RunnableConfig."""
    if not config:
        return None

    configurable = config.get("configurable", {})
    value = configurable.get(key)

    if isinstance(value, str):
        try:
            return json.loads(value)
        except json.JSONDecodeError:
            return value

    return value


class IdentifyOldMemoriesNode:
    """Node for identifying old episodic memories that need consolidation."""

    def __init__(self, age_threshold_days: int = 30) -> None:
        self.name = "identify_old_memories"
        self.age_threshold_days = age_threshold_days
        self.memory_bridge = MemoryBridge.from_env()

    async def __call__(
        self,
        state: MemoryMaintenanceState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Identify old memories that should be consolidated."""
        logger.debug("Running identify_old_memories node")

        try:
            # Calculate cutoff date
            cutoff_date = datetime.utcnow() - timedelta(days=self.age_threshold_days)

            old_memories = state.get("old_memories", [])

            if not old_memories:
                old_memories = await self._fetch_old_memories(cutoff_date, config)

            logger.info(f"Found {len(old_memories)} old memories for consolidation")

            return {
                "old_memories": old_memories,
                "status": "running",
            }

        except Exception as e:
            logger.exception("Failed to identify old memories")
            return {
                "errors": [f"identify_old_memories: {str(e)}"],
                "status": "error",
            }

    async def _fetch_old_memories(
        self,
        cutoff_date: datetime,
        config: Optional[RunnableConfig],
    ) -> List[Dict[str, Any]]:
        """Fetch memories older than the cutoff date from workflow metadata."""
        configured_memories = _load_configurable_payload(config, "old_memories")
        if not isinstance(configured_memories, list):
            configured_memories = _load_configurable_payload(config, "memory_entries")

        if isinstance(configured_memories, list):
            old_memories: List[Dict[str, Any]] = []
            for index, item in enumerate(configured_memories):
                if isinstance(item, str):
                    memory = {
                        "id": f"memory-{index}",
                        "content": item,
                        "timestamp": cutoff_date.isoformat(),
                    }
                elif isinstance(item, dict):
                    memory = {
                        "id": item.get("id", f"memory-{index}"),
                        "content": item.get("content", ""),
                        "timestamp": item.get("timestamp", cutoff_date.isoformat()),
                        "namespace": item.get("namespace"),
                        "user_id": item.get("user_id"),
                        "importance": item.get("importance"),
                    }
                else:
                    continue

                timestamp = memory.get("timestamp", cutoff_date.isoformat())
                try:
                    parsed_timestamp = datetime.fromisoformat(str(timestamp).replace("Z", "+00:00"))
                except Exception:
                    parsed_timestamp = cutoff_date

                if parsed_timestamp <= cutoff_date:
                    old_memories.append(memory)

            return old_memories

        if self.memory_bridge is None:
            return []

        namespace = _load_configurable_payload(config, "namespace")
        user_id = _load_configurable_payload(config, "user_id")
        limit = _load_configurable_payload(config, "limit")
        bridge_results = await self.memory_bridge.fetch_old_memories(
            age_days=self.age_threshold_days,
            namespace=namespace if isinstance(namespace, str) and namespace else None,
            user_id=user_id if isinstance(user_id, str) and user_id else None,
            limit=limit if isinstance(limit, int) and limit > 0 else 100,
        )
        return bridge_results


class SummarizeMemoriesNode:
    """Node for summarizing old memories into archive entries."""

    def __init__(self, model: Optional[str] = None) -> None:
        self.name = "summarize_memories"
        self.model = model or "gpt-4"
        self._llm: Optional[Any] = None

    def _get_llm(self) -> Any:
        """Get or create LLM instance."""
        if self._llm is None:
            try:
                from langchain_openai import ChatOpenAI
                self._llm = ChatOpenAI(
                    model=self.model,
                    temperature=0.3,  # Lower temp for consistency
                    max_tokens=1000,
                )
            except Exception as e:
                logger.error(f"Failed to initialize OpenAI model: {e}")
                self._llm = None
        return self._llm

    async def __call__(
        self,
        state: MemoryMaintenanceState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Summarize old memories into archive entries."""
        logger.debug("Running summarize_memories node")

        try:
            old_memories = state.get("old_memories", [])
            if not old_memories:
                logger.info("No old memories to summarize")
                return {"summaries": [], "consolidated_count": 0}

            summaries = []

            # Group memories by theme/time period for better summarization
            memory_groups = self._group_memories(old_memories)

            for group in memory_groups:
                summary = await self._summarize_group(group)
                if summary:
                    summaries.append(summary)

            return {
                "summaries": summaries,
                "consolidated_count": len(summaries),
            }

        except Exception as e:
            logger.exception("Failed to summarize memories")
            return {
                "errors": state.get("errors", []) + [f"summarize_memories: {str(e)}"],
            }

    def _group_memories(
        self,
        memories: List[Dict[str, Any]],
    ) -> List[List[Dict[str, Any]]]:
        """Group memories by time period or similarity for summarization."""
        if not memories:
            return []

        # Simple grouping by week
        groups: Dict[str, List[Dict[str, Any]]] = {}

        for memory in memories:
            timestamp = memory.get("timestamp", "")
            if timestamp:
                try:
                    dt = datetime.fromisoformat(timestamp.replace("Z", "+00:00"))
                    week_key = dt.strftime("%Y-W%W")
                except Exception:
                    week_key = "unknown"
            else:
                week_key = "unknown"

            if week_key not in groups:
                groups[week_key] = []
            groups[week_key].append(memory)

        return list(groups.values())

    async def _summarize_group(
        self,
        group: List[Dict[str, Any]],
    ) -> Optional[Dict[str, Any]]:
        """Summarize a group of related memories."""
        if not group:
            return None

        llm = self._get_llm()

        # Prepare content for summarization
        contents = [m.get("content", "") for m in group if m.get("content")]
        combined_content = "\n\n".join(contents)

        if llm is None:
            # Fallback mock summarization
            return {
                "id": f"summary_{hash(combined_content) & 0xFFFFFFFF}",
                "original_count": len(group),
                "summary": f"Mock summary of {len(group)} memories",
                "key_points": ["Mock point 1", "Mock point 2"],
                "source_memory_ids": [m.get("id") for m in group if m.get("id")],
                "namespace": group[0].get("namespace") if group else None,
                "importance": max(
                    [float(m.get("importance", 0.5)) for m in group if m.get("importance") is not None]
                    or [0.5]
                ),
                "source_type": "conversation",
                "time_range": {
                    "start": group[0].get("timestamp", ""),
                    "end": group[-1].get("timestamp", ""),
                },
                "embedding": None,
            }

        # Create summarization prompt
        prompt = f"""Please summarize the following memories into a concise archive entry:

Memories:
{combined_content[:4000]}  # Limit content length

Provide:
1. A brief summary (2-3 sentences)
2. Key points (bullet list)
3. Overall sentiment/tone

Format as JSON with keys: summary, key_points, sentiment"""

        try:
            messages = [
                SystemMessage(content="You are a memory summarization assistant."),
                HumanMessage(content=prompt),
            ]

            response = await llm.ainvoke(messages)

            # Parse response
            try:
                parsed = json.loads(response.content)
            except json.JSONDecodeError:
                # Fallback if not valid JSON
                parsed = {
                    "summary": response.content[:500],
                    "key_points": [],
                    "sentiment": "neutral",
                }

            return {
                "id": f"summary_{hash(combined_content) & 0xFFFFFFFF}",
                "original_count": len(group),
                "summary": parsed.get("summary", ""),
                "key_points": parsed.get("key_points", []),
                "sentiment": parsed.get("sentiment", "neutral"),
                "source_memory_ids": [m.get("id") for m in group if m.get("id")],
                "namespace": group[0].get("namespace") if group else None,
                "importance": max(
                    [float(m.get("importance", 0.5)) for m in group if m.get("importance") is not None]
                    or [0.5]
                ),
                "source_type": "conversation",
                "time_range": {
                    "start": group[0].get("timestamp", ""),
                    "end": group[-1].get("timestamp", ""),
                },
                "embedding": None,  # Will be generated in next step
            }

        except Exception as e:
            logger.error(f"Failed to summarize group: {e}")
            return None


class UpdateEmbeddingsNode:
    """Node for updating vector embeddings for consolidated memories."""

    def __init__(self, embedding_model: Optional[str] = None) -> None:
        self.name = "update_embeddings"
        self.embedding_model = embedding_model or "text-embedding-3-small"
        self._embeddings: Optional[Any] = None

    def _get_embeddings(self) -> Any:
        """Get or create embeddings instance."""
        if self._embeddings is None:
            try:
                from langchain_openai import OpenAIEmbeddings
                self._embeddings = OpenAIEmbeddings(
                    model=self.embedding_model,
                )
            except Exception as e:
                logger.error(f"Failed to initialize embeddings: {e}")
                self._embeddings = None
        return self._embeddings

    async def __call__(
        self,
        state: MemoryMaintenanceState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Generate embeddings for consolidated memories."""
        logger.debug("Running update_embeddings node")

        try:
            summaries = state.get("summaries", [])
            if not summaries:
                return {"summaries": summaries}

            embeddings = self._get_embeddings()

            updated_summaries = []
            for summary in summaries:
                try:
                    text_to_embed = f"{summary.get('summary', '')} {' '.join(summary.get('key_points', []))}"

                    if embeddings:
                        embedding_vector = await embeddings.aembed_query(text_to_embed)
                        summary["embedding"] = embedding_vector
                    else:
                        # Mock embedding
                        summary["embedding"] = [0.0] * 1536  # Standard OpenAI embedding size

                    updated_summaries.append(summary)

                except Exception as e:
                    logger.error(f"Failed to generate embedding for summary: {e}")
                    summary["embedding"] = None
                    updated_summaries.append(summary)

            return {"summaries": updated_summaries}

        except Exception as e:
            logger.exception("Failed to update embeddings")
            return {
                "errors": state.get("errors", []) + [f"update_embeddings: {str(e)}"],
            }


class ArchiveMemoriesNode:
    """Node for archiving old memories and storing summaries."""

    def __init__(self) -> None:
        self.name = "archive_memories"
        self.memory_bridge = MemoryBridge.from_env()

    async def __call__(
        self,
        state: MemoryMaintenanceState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Archive old memories and store summaries."""
        logger.debug("Running archive_memories node")

        try:
            summaries = state.get("summaries", [])
            old_memories = state.get("old_memories", [])

            archived_count = 0

            stored_archive_entries = list(state.get("archive_entries", []))
            for summary in summaries:
                try:
                    stored_archive_entries.append(await self._store_archive_entry(summary))
                    archived_count += 1
                except Exception as e:
                    logger.error(f"Failed to store archive entry: {e}")

            archived_memory_ids = list(state.get("archived_memory_ids", []))
            memory_ids_to_archive = [
                memory.get("id") for memory in old_memories if memory.get("id")
            ]
            if memory_ids_to_archive:
                try:
                    archived_memory_ids.extend(
                        await self._mark_memories_archived(memory_ids_to_archive)
                    )
                except Exception as e:
                    logger.error(f"Failed to mark memories as archived: {e}")

            return {
                "archived_count": archived_count,
                "archive_entries": stored_archive_entries,
                "archived_memory_ids": archived_memory_ids,
                "status": "completed" if not state.get("errors") else "error",
            }

        except Exception as e:
            logger.exception("Failed to archive memories")
            return {
                "errors": state.get("errors", []) + [f"archive_memories: {str(e)}"],
                "status": "error",
            }

    async def _store_archive_entry(self, summary: Dict[str, Any]) -> Dict[str, Any]:
        """Persist an archive entry when a Rust bridge is available."""
        logger.debug(f"Storing archive entry: {summary.get('id')}")
        entry = {
            "id": summary.get("id"),
            "summary": summary.get("summary", ""),
            "key_points": summary.get("key_points", []),
            "source_memory_ids": summary.get("source_memory_ids", []),
            "namespace": summary.get("namespace"),
            "importance": summary.get("importance"),
            "source_type": summary.get("source_type"),
            "time_range": summary.get("time_range", {}),
            "embedding": summary.get("embedding"),
        }
        if self.memory_bridge is None:
            return entry

        result = await self.memory_bridge.store_archive_entry(entry)
        if isinstance(result, dict):
            entry["stored"] = bool(result.get("stored", True))
            if result.get("id"):
                entry["id"] = result["id"]
        return entry

    async def _mark_memories_archived(self, memory_ids: List[str]) -> List[str]:
        """Archive original memory ids in the Rust store when available."""
        if not memory_ids:
            return []
        logger.debug("Marking %s memories as archived", len(memory_ids))
        if self.memory_bridge is None:
            return memory_ids

        result = await self.memory_bridge.archive_memory_ids(memory_ids)
        archived_ids = result.get("memory_ids", [])
        if isinstance(archived_ids, list):
            return [str(memory_id) for memory_id in archived_ids]
        return memory_ids


def has_memories_to_process(state: MemoryMaintenanceState) -> str:
    """Determine if there are memories to process."""
    old_memories = state.get("old_memories", [])
    errors = state.get("errors", [])

    if errors:
        return "end"

    if not old_memories:
        return "end"

    return "summarize"


def has_errors(state: MemoryMaintenanceState) -> str:
    """Check if there were errors during processing."""
    errors = state.get("errors", [])
    return "end" if errors else "continue"


def build_memory_maintenance_graph(
    age_threshold_days: int = 30,
    model: Optional[str] = None,
    embedding_model: Optional[str] = None,
) -> StateGraph:
    """Build the memory maintenance and consolidation graph.

    The graph follows this flow:
    1. identify_old_memories - Find episodic memories older than threshold
    2. summarize_memories - Group and summarize related memories
    3. update_embeddings - Generate vector embeddings for summaries
    4. archive_memories - Store summaries and archive originals
    """
    # Create nodes
    identify = IdentifyOldMemoriesNode(age_threshold_days=age_threshold_days)
    summarize = SummarizeMemoriesNode(model=model)
    update_embeddings = UpdateEmbeddingsNode(embedding_model=embedding_model)
    archive = ArchiveMemoriesNode()

    # Build graph
    workflow = StateGraph(MemoryMaintenanceState)

    # Add nodes
    workflow.add_node("identify_old_memories", identify)
    workflow.add_node("summarize_memories", summarize)
    workflow.add_node("update_embeddings", update_embeddings)
    workflow.add_node("archive_memories", archive)

    # Add edges
    workflow.add_edge(START, "identify_old_memories")

    # Conditional from identify based on whether there are memories
    workflow.add_conditional_edges(
        "identify_old_memories",
        has_memories_to_process,
        {
            "summarize": "summarize_memories",
            "end": "archive_memories",
        },
    )

    workflow.add_edge("summarize_memories", "update_embeddings")

    # Conditional from embeddings to archive or end on error
    workflow.add_conditional_edges(
        "update_embeddings",
        has_errors,
        {
            "continue": "archive_memories",
            "end": "archive_memories",
        },
    )

    workflow.add_edge("archive_memories", END)

    return workflow.compile()
