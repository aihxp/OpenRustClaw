"""LangSmith tracing and evaluation bridge."""

import json
import logging
import os
from contextlib import contextmanager
from typing import Any, Dict, Generator, List, Optional
from uuid import uuid4

logger = logging.getLogger(__name__)


class LangSmithBridge:
    """Bridge for LangSmith tracing from the sidecar."""

    def __init__(self) -> None:
        self._current_trace_id: str = ""
        self._client: Optional[Any] = None
        self._enabled = self._check_enabled()

        if self._enabled:
            try:
                from langsmith import Client
                self._client = Client()
                logger.info("LangSmith client initialized successfully")
            except Exception as e:
                logger.warning(f"Failed to initialize LangSmith client: {e}")
                self._enabled = False

    def _check_enabled(self) -> bool:
        """Check if LangSmith is properly configured."""
        api_key = os.getenv("LANGSMITH_API_KEY") or os.getenv("LANGCHAIN_API_KEY")
        return api_key is not None and api_key != ""

    @property
    def enabled(self) -> bool:
        """Check if LangSmith integration is enabled."""
        return self._enabled and self._client is not None

    def get_current_trace_id(self) -> str:
        """Get the current trace ID."""
        return self._current_trace_id

    @contextmanager
    def trace(
        self,
        workflow_name: str,
        thread_id: str,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> Generator[None, None, None]:
        """Create a LangSmith trace context for workflow execution."""
        if not self.enabled:
            yield
            return

        from langsmith.run_trees import RunTree

        self._current_trace_id = str(uuid4())
        run_tree = RunTree(
            name=workflow_name,
            run_type="chain",
            inputs={"thread_id": thread_id, **(metadata or {})},
            project_name=os.getenv("LANGSMITH_PROJECT", "openrustclaw"),
        )

        try:
            yield
            run_tree.end(outputs={"status": "completed"})
            self._client.create_run(
                name=run_tree.name,
                run_type=run_tree.run_type,
                inputs=run_tree.inputs,
                outputs=run_tree.outputs,
                project_name=run_tree.project_name,
            )
        except Exception as e:
            run_tree.end(error=str(e))
            logger.warning(f"Error in LangSmith trace: {e}")
            raise
        finally:
            self._current_trace_id = ""

    def trace_node(
        self,
        node_name: str,
        inputs: Dict[str, Any],
        outputs: Optional[Dict[str, Any]] = None,
        error: Optional[str] = None,
    ) -> None:
        """Trace an individual node execution."""
        if not self.enabled:
            return

        try:
            run_outputs = outputs or {}
            if error:
                run_outputs["error"] = error

            self._client.create_run(
                name=node_name,
                run_type="tool" if "tool" in node_name.lower() else "chain",
                inputs=inputs,
                outputs=run_outputs,
                error=error,
            )
        except Exception as e:
            logger.warning(f"Failed to trace node {node_name}: {e}")

    def export_traces(
        self,
        workflow_id: Optional[str] = None,
        limit: int = 100,
    ) -> List[Dict[str, Any]]:
        """Export traces for analysis or debugging."""
        if not self.enabled or self._client is None:
            logger.warning("LangSmith not enabled, cannot export traces")
            return []

        try:
            runs = self._client.list_runs(
                project_name=os.getenv("LANGSMITH_PROJECT", "openrustclaw"),
                filter=f'eq(name, "{workflow_id}")' if workflow_id else None,
                limit=limit,
            )
            return [self._run_to_dict(run) for run in runs]
        except Exception as e:
            logger.error(f"Failed to export traces: {e}")
            return []

    def _run_to_dict(self, run: Any) -> Dict[str, Any]:
        """Convert a LangSmith run to a dictionary."""
        return {
            "id": str(run.id) if hasattr(run, "id") else "",
            "name": run.name if hasattr(run, "name") else "",
            "run_type": run.run_type if hasattr(run, "run_type") else "",
            "inputs": run.inputs if hasattr(run, "inputs") else {},
            "outputs": run.outputs if hasattr(run, "outputs") else {},
            "error": run.error if hasattr(run, "error") else None,
            "start_time": run.start_time.isoformat() if hasattr(run, "start_time") and run.start_time else None,
            "end_time": run.end_time.isoformat() if hasattr(run, "end_time") and run.end_time else None,
            "latency": self._calculate_latency(run),
        }

    def _calculate_latency(self, run: Any) -> Optional[float]:
        """Calculate latency in seconds for a run."""
        try:
            if hasattr(run, "start_time") and hasattr(run, "end_time"):
                if run.start_time and run.end_time:
                    return (run.end_time - run.start_time).total_seconds()
        except Exception:
            pass
        return None

    def get_metrics(
        self,
        workflow_id: Optional[str] = None,
        time_range_hours: int = 24,
    ) -> Dict[str, Any]:
        """Get aggregated metrics for workflows."""
        if not self.enabled:
            return {"enabled": False}

        traces = self.export_traces(workflow_id, limit=1000)

        if not traces:
            return {"enabled": True, "count": 0}

        total_latency = 0.0
        error_count = 0
        latencies = []

        for trace in traces:
            if trace.get("latency"):
                latencies.append(trace["latency"])
                total_latency += trace["latency"]
            if trace.get("error"):
                error_count += 1

        count = len(traces)
        return {
            "enabled": True,
            "count": count,
            "error_count": error_count,
            "error_rate": error_count / count if count > 0 else 0,
            "avg_latency": total_latency / count if count > 0 else 0,
            "min_latency": min(latencies) if latencies else 0,
            "max_latency": max(latencies) if latencies else 0,
            "workflow_id": workflow_id,
        }

    def create_feedback(
        self,
        run_id: str,
        key: str,
        score: Optional[float] = None,
        value: Optional[Any] = None,
        comment: Optional[str] = None,
    ) -> bool:
        """Create feedback for a run."""
        if not self.enabled or self._client is None:
            logger.warning("LangSmith not enabled, cannot create feedback")
            return False

        try:
            self._client.create_feedback(
                run_id=run_id,
                key=key,
                score=score,
                value=value,
                comment=comment,
            )
            return True
        except Exception as e:
            logger.error(f"Failed to create feedback: {e}")
            return False

    def log_evaluation_result(
        self,
        workflow_id: str,
        evaluation_name: str,
        score: float,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> bool:
        """Log an evaluation result to LangSmith."""
        if not self.enabled:
            return False

        try:
            # Create a special run for evaluation
            self._client.create_run(
                name=f"eval:{evaluation_name}",
                run_type="evaluation",
                inputs={"workflow_id": workflow_id},
                outputs={
                    "score": score,
                    "metadata": metadata or {},
                },
                tags=["evaluation", workflow_id],
            )
            return True
        except Exception as e:
            logger.error(f"Failed to log evaluation: {e}")
            return False
