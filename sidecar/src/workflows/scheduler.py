"""Generic scheduled workflow executor with idempotency and retry logic."""

import hashlib
import json
import logging
import time
from datetime import datetime, timedelta
from typing import Annotated, Any, Dict, List, Literal, Optional, Sequence, TypedDict

from langchain_core.messages import BaseMessage
from langchain_core.runnables import RunnableConfig
from langgraph.graph import END, START, StateGraph
from langgraph.graph.message import add_messages

logger = logging.getLogger(__name__)


class SchedulerState(TypedDict):
    """State for the scheduler workflow."""

    messages: Annotated[Sequence[BaseMessage], add_messages]
    job_id: str
    job_type: str
    payload: Dict[str, Any]
    scheduled_time: str  # ISO format
    idempotency_key: str
    retry_count: int
    max_retries: int
    retry_delay_base: float  # Base delay in seconds for exponential backoff
    status: Literal["pending", "checking", "executing", "retrying", "completed", "failed", "skipped"]
    result: Optional[Dict[str, Any]]
    error: Optional[str]
    execution_time_ms: float
    previous_attempts: List[Dict[str, Any]]


def create_default_state() -> SchedulerState:
    """Create default initial state."""
    return {
        "messages": [],
        "job_id": "",
        "job_type": "",
        "payload": {},
        "scheduled_time": "",
        "idempotency_key": "",
        "retry_count": 0,
        "max_retries": 3,
        "retry_delay_base": 2.0,
        "status": "pending",
        "result": None,
        "error": None,
        "execution_time_ms": 0.0,
        "previous_attempts": [],
    }


class IdempotencyCheckNode:
    """Node for checking if job has already been executed."""

    def __init__(self) -> None:
        self.name = "idempotency_check"
        self._executed_jobs: Dict[str, Dict[str, Any]] = {}  # In-memory cache, use Redis in prod

    async def __call__(
        self,
        state: SchedulerState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Check if job has already been executed."""
        logger.debug("Running idempotency_check node")

        try:
            idempotency_key = state.get("idempotency_key", "")
            job_id = state.get("job_id", "")

            # Generate key if not provided
            if not idempotency_key and job_id:
                idempotency_key = self._generate_idempotency_key(state)

            if not idempotency_key:
                # No idempotency key, proceed with execution
                return {
                    "idempotency_key": idempotency_key,
                    "status": "checking",
                }

            # Check if already executed
            existing = await self._check_executed(idempotency_key)

            if existing:
                logger.info(f"Job {job_id} already executed, returning cached result")
                return {
                    "status": "skipped",
                    "result": existing.get("result"),
                    "error": None,
                }

            return {
                "idempotency_key": idempotency_key,
                "status": "checking",
            }

        except Exception as e:
            logger.exception("Idempotency check failed")
            return {
                "error": f"Idempotency check failed: {str(e)}",
                "status": "failed",
            }

    def _generate_idempotency_key(self, state: SchedulerState) -> str:
        """Generate idempotency key from job details."""
        job_id = state.get("job_id", "")
        job_type = state.get("job_type", "")
        scheduled_time = state.get("scheduled_time", "")

        key_data = f"{job_id}:{job_type}:{scheduled_time}"
        return hashlib.sha256(key_data.encode()).hexdigest()[:32]

    async def _check_executed(self, idempotency_key: str) -> Optional[Dict[str, Any]]:
        """Check if job has been executed."""
        # In production, check Redis/database
        return self._executed_jobs.get(idempotency_key)

    async def mark_executed(
        self,
        idempotency_key: str,
        result: Dict[str, Any],
    ) -> None:
        """Mark job as executed."""
        self._executed_jobs[idempotency_key] = {
            "result": result,
            "executed_at": datetime.utcnow().isoformat(),
        }


class ScheduleValidationNode:
    """Node for validating schedule time and conditions."""

    def __init__(self) -> None:
        self.name = "schedule_validation"

    async def __call__(
        self,
        state: SchedulerState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Validate schedule time and preconditions."""
        logger.debug("Running schedule_validation node")

        try:
            scheduled_time_str = state.get("scheduled_time", "")
            job_id = state.get("job_id", "")

            if not scheduled_time_str:
                # No scheduled time, execute immediately
                return {"status": "executing"}

            # Parse scheduled time
            try:
                scheduled_time = datetime.fromisoformat(scheduled_time_str.replace("Z", "+00:00"))
            except ValueError:
                return {
                    "error": f"Invalid scheduled_time format: {scheduled_time_str}",
                    "status": "failed",
                }

            # Check if it's time to execute
            now = datetime.utcnow()
            if scheduled_time.tzinfo:
                now = datetime.now(scheduled_time.tzinfo)

            if scheduled_time > now:
                # Not yet time to execute
                wait_seconds = (scheduled_time - now).total_seconds()
                logger.info(f"Job {job_id} scheduled for {scheduled_time}, waiting {wait_seconds}s")
                return {
                    "status": "pending",
                    "error": f"Scheduled for future: {scheduled_time.isoformat()}",
                }

            # Check additional preconditions
            preconditions_met = await self._check_preconditions(state)
            if not preconditions_met:
                return {
                    "status": "pending",
                    "error": "Preconditions not met",
                }

            return {"status": "executing"}

        except Exception as e:
            logger.exception("Schedule validation failed")
            return {
                "error": str(e),
                "status": "failed",
            }

    async def _check_preconditions(self, state: SchedulerState) -> bool:
        """Check if all preconditions for execution are met."""
        # In production, check external conditions (DB state, dependencies, etc.)
        return True


class ExecuteJobNode:
    """Node for executing the scheduled job."""

    def __init__(self) -> None:
        self.name = "execute_job"
        self._job_handlers: Dict[str, Any] = {}

    def register_handler(self, job_type: str, handler: Any) -> None:
        """Register a handler for a job type."""
        self._job_handlers[job_type] = handler

    async def __call__(
        self,
        state: SchedulerState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Execute the scheduled job."""
        logger.debug("Running execute_job node")

        start_time = time.time()
        job_id = state.get("job_id", "")
        job_type = state.get("job_type", "")
        payload = state.get("payload", {})

        try:
            # Get job handler
            handler = self._job_handlers.get(job_type)

            if not handler:
                # Default handler - execute as function call
                result = await self._execute_default(payload)
            else:
                # Use registered handler
                result = await handler(payload)

            execution_time_ms = (time.time() - start_time) * 1000

            logger.info(f"Job {job_id} executed successfully in {execution_time_ms:.2f}ms")

            return {
                "result": result,
                "execution_time_ms": execution_time_ms,
                "status": "completed",
                "error": None,
            }

        except Exception as e:
            logger.exception(f"Job {job_id} execution failed")
            execution_time_ms = (time.time() - start_time) * 1000

            return {
                "error": str(e),
                "execution_time_ms": execution_time_ms,
                "status": "failed",
            }

    async def _execute_default(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Default job execution handler."""
        action = payload.get("action", "unknown")

        # Handle common actions
        if action == "notify":
            return {"action": "notify", "status": "sent", "recipient": payload.get("to")}

        elif action == "process":
            return {"action": "process", "status": "processed", "items": payload.get("items", [])}

        elif action == "cleanup":
            return {"action": "cleanup", "status": "cleaned", "target": payload.get("target")}

        return {"action": action, "status": "executed"}


class RetryLogicNode:
    """Node for handling retry logic with exponential backoff."""

    def __init__(self) -> None:
        self.name = "retry_logic"

    async def __call__(
        self,
        state: SchedulerState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Handle retry logic for failed jobs."""
        logger.debug("Running retry_logic node")

        try:
            retry_count = state.get("retry_count", 0)
            max_retries = state.get("max_retries", 3)
            error = state.get("error", "")
            previous_attempts = state.get("previous_attempts", [])

            # Record this attempt
            previous_attempts.append({
                "timestamp": datetime.utcnow().isoformat(),
                "error": error,
                "retry_count": retry_count,
            })

            if retry_count >= max_retries:
                logger.error(f"Job failed after {max_retries} retries")
                return {
                    "status": "failed",
                    "previous_attempts": previous_attempts,
                    "error": f"Max retries ({max_retries}) exceeded. Last error: {error}",
                }

            # Calculate backoff delay
            retry_delay_base = state.get("retry_delay_base", 2.0)
            delay = retry_delay_base * (2 ** retry_count)

            logger.info(f"Retrying job, attempt {retry_count + 1}/{max_retries} after {delay}s delay")

            # In production, this would schedule a retry via message queue
            # For now, we simulate the delay
            await self._schedule_retry(state, delay)

            return {
                "retry_count": retry_count + 1,
                "status": "retrying",
                "previous_attempts": previous_attempts,
                "retry_after_seconds": delay,
            }

        except Exception as e:
            logger.exception("Retry logic failed")
            return {
                "error": f"Retry logic failed: {str(e)}",
                "status": "failed",
            }

    async def _schedule_retry(self, state: SchedulerState, delay: float) -> None:
        """Schedule a job retry after delay."""
        # In production, use a message queue or scheduler
        # This is a placeholder
        logger.debug(f"Would schedule retry after {delay}s")


class CompletionNode:
    """Node for finalizing job execution and cleanup."""

    def __init__(self, idempotency_node: IdempotencyCheckNode) -> None:
        self.name = "completion"
        self.idempotency_node = idempotency_node

    async def __call__(
        self,
        state: SchedulerState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Finalize job execution."""
        logger.debug("Running completion node")

        try:
            status = state.get("status", "")
            idempotency_key = state.get("idempotency_key", "")
            result = state.get("result", {})

            # Mark as executed if completed successfully
            if status == "completed" and idempotency_key:
                await self.idempotency_node.mark_executed(
                    idempotency_key,
                    result or {},
                )

            return {
                "status": status,
                "completed_at": datetime.utcnow().isoformat(),
            }

        except Exception as e:
            logger.exception("Completion failed")
            return {
                "error": str(e),
                "status": "failed",
            }


def should_execute(state: SchedulerState) -> str:
    """Determine if job should execute or was already processed."""
    status = state.get("status", "")

    if status == "skipped":
        return "complete"  # Already executed

    if status in ["failed", "error"]:
        return "error"

    return "execute"


def should_retry(state: SchedulerState) -> str:
    """Determine if job should be retried or marked as failed."""
    status = state.get("status", "")
    retry_count = state.get("retry_count", 0)
    max_retries = state.get("max_retries", 3)

    if status == "completed":
        return "complete"

    if retry_count >= max_retries:
        return "fail"

    return "retry"


def build_scheduler_graph(
    max_retries: int = 3,
    retry_delay_base: float = 2.0,
) -> StateGraph:
    """Build the scheduled workflow executor graph.

    The graph follows this flow:
    1. idempotency_check - Check if already executed
    2. schedule_validation - Validate timing and preconditions
    3. execute_job - Execute the job
    4. retry_logic - Handle failures with exponential backoff
    5. completion - Finalize and mark as executed

    Args:
        max_retries: Maximum number of retry attempts
        retry_delay_base: Base delay in seconds for exponential backoff

    Returns:
        Compiled StateGraph
    """
    # Create nodes
    idempotency = IdempotencyCheckNode()
    validation = ScheduleValidationNode()
    executor = ExecuteJobNode()
    retry = RetryLogicNode()
    completion = CompletionNode(idempotency)

    # Build graph
    workflow = StateGraph(SchedulerState)

    # Add nodes
    workflow.add_node("idempotency_check", idempotency)
    workflow.add_node("schedule_validation", validation)
    workflow.add_node("execute_job", executor)
    workflow.add_node("retry_logic", retry)
    workflow.add_node("completion", completion)

    # Add edges
    workflow.add_edge(START, "idempotency_check")

    # From idempotency check
    workflow.add_conditional_edges(
        "idempotency_check",
        should_execute,
        {
            "execute": "schedule_validation",
            "complete": "completion",
            "error": "completion",
        },
    )

    workflow.add_edge("schedule_validation", "execute_job")

    # From execution to retry logic or completion
    workflow.add_conditional_edges(
        "execute_job",
        should_retry,
        {
            "complete": "completion",
            "retry": "retry_logic",
            "fail": "completion",
        },
    )

    # From retry back to validation for retry
    workflow.add_edge("retry_logic", "schedule_validation")

    workflow.add_edge("completion", END)

    return workflow.compile()
