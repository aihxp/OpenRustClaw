"""gRPC server entry point for the OpenRustClaw Python sidecar."""

import argparse
import asyncio
import json
import logging
import os
import threading
import uuid
from concurrent import futures
from typing import Any, AsyncIterator, Callable, Dict, Optional

import grpc

from .proto import orchestration_pb2
from .proto import orchestration_pb2_grpc
from .langsmith_bridge import LangSmithBridge
from .workflow_contract import WorkflowContractError, parse_workflow_request

# Import workflow builders
from .workflows.agent_orchestrator import build_agent_graph
from .workflows.memory_maintenance import build_memory_maintenance_graph
from .workflows.rag_pipeline import build_rag_graph
from .workflows.scheduler import build_scheduler_graph
from .workflows.reminder import build_reminder_graph

logger = logging.getLogger(__name__)


class WorkflowRegistry:
    """Thread-safe registry for managing workflow executions."""

    def __init__(self) -> None:
        self._workflows: Dict[str, Dict[str, Any]] = {}
        self._lock = threading.RLock()

    def register(
        self,
        thread_id: str,
        workflow_id: str,
        status: str = "pending",
        current_step: str = "",
    ) -> None:
        """Register a new workflow execution."""
        with self._lock:
            self._workflows[thread_id] = {
                "workflow_id": workflow_id,
                "status": status,
                "current_step": current_step,
                "steps_completed": 0,
                "output": None,
                "error": None,
                "trace_id": "",
            }

    def update(
        self,
        thread_id: str,
        status: Optional[str] = None,
        current_step: Optional[str] = None,
        increment_steps: bool = False,
        output: Optional[str] = None,
        error: Optional[str] = None,
        trace_id: Optional[str] = None,
    ) -> None:
        """Update workflow execution status."""
        with self._lock:
            if thread_id not in self._workflows:
                return
            if status is not None:
                self._workflows[thread_id]["status"] = status
            if current_step is not None:
                self._workflows[thread_id]["current_step"] = current_step
            if increment_steps:
                self._workflows[thread_id]["steps_completed"] += 1
            if output is not None:
                self._workflows[thread_id]["output"] = output
            if error is not None:
                self._workflows[thread_id]["error"] = error
            if trace_id is not None:
                self._workflows[thread_id]["trace_id"] = trace_id

    def get(self, thread_id: str) -> Optional[Dict[str, Any]]:
        """Get workflow execution status."""
        with self._lock:
            return self._workflows.get(thread_id)

    def remove(self, thread_id: str) -> None:
        """Remove a workflow from registry."""
        with self._lock:
            if thread_id in self._workflows:
                del self._workflows[thread_id]


class OrchestrationServicer(orchestration_pb2_grpc.OrchestrationServiceServicer):
    """Handles LangGraph workflow execution requests from Rust."""

    def __init__(self) -> None:
        self.registry = WorkflowRegistry()
        self.langsmith = LangSmithBridge()
        self._workflows: Dict[str, Callable] = {
            "agent": build_agent_graph,
            "memory_maintenance": build_memory_maintenance_graph,
            "rag": build_rag_graph,
            "scheduler": build_scheduler_graph,
            "reminder": build_reminder_graph,
        }

    async def ExecuteWorkflow(
        self,
        request: orchestration_pb2.WorkflowRequest,
        context: grpc.ServicerContext,
    ) -> orchestration_pb2.WorkflowResponse:
        """Execute a LangGraph workflow."""
        logger.info(f"Executing workflow: {request.workflow_id} for thread: {request.thread_id}")

        try:
            parsed_request = parse_workflow_request(request)
        except WorkflowContractError as exc:
            logger.error("Invalid workflow request: %s", exc)
            return orchestration_pb2.WorkflowResponse(
                thread_id=request.thread_id or "",
                output="{}",
                status="error",
                error=str(exc),
                trace_id="",
            )

        thread_id = parsed_request.thread_id

        # Register workflow
        self.registry.register(thread_id, request.workflow_id)

        try:
            # Get workflow builder
            workflow_builder = self._workflows.get(request.workflow_id)
            if not workflow_builder:
                error_msg = f"Unknown workflow: {request.workflow_id}"
                logger.error(error_msg)
                self.registry.update(thread_id, status="error", error=error_msg)
                return orchestration_pb2.WorkflowResponse(
                    thread_id=thread_id,
                    output="{}",
                    status="error",
                    error=error_msg,
                    trace_id="",
                )

            # Build and execute workflow with LangSmith tracing
            self.registry.update(thread_id, status="running", current_step="building_graph")

            with self.langsmith.trace(
                request.workflow_id, thread_id, parsed_request.metadata
            ) as trace_id:
                graph = workflow_builder()
                self.registry.update(thread_id, current_step="executing")
                result = await self._execute_graph(
                    graph,
                    parsed_request.input_data,
                    parsed_request.runnable_config(),
                    thread_id,
                )

            # Determine status
            status = result.get("status", "completed")
            error_msg = result.get("error", "")
            resolved_trace_id = trace_id or self.langsmith.get_last_trace_id()

            self.registry.update(
                thread_id,
                status=status,
                output=json.dumps(result),
                error=error_msg,
                trace_id=resolved_trace_id,
            )

            return orchestration_pb2.WorkflowResponse(
                thread_id=thread_id,
                output=json.dumps(result),
                status=status,
                error=error_msg,
                trace_id=resolved_trace_id,
            )

        except Exception as e:
            logger.exception("Workflow execution failed")
            error_msg = str(e)
            self.registry.update(thread_id, status="error", error=error_msg)
            return orchestration_pb2.WorkflowResponse(
                thread_id=thread_id,
                output="{}",
                status="error",
                error=error_msg,
                trace_id=self.langsmith.get_current_trace_id() or self.langsmith.get_last_trace_id(),
            )

    async def _execute_graph(
        self,
        graph: Any,
        input_data: Dict[str, Any],
        config: Dict[str, Any],
        thread_id: str,
    ) -> Dict[str, Any]:
        """Execute a LangGraph with progress tracking."""
        from langchain_core.runnables import RunnableConfig

        lc_config = RunnableConfig(**config)

        # Collect results
        results = []
        async for event in graph.astream(input_data, config=lc_config):
            for node_name, node_output in event.items():
                results.append(node_output)
                self.registry.update(
                    thread_id,
                    current_step=node_name,
                    increment_steps=True,
                )
                self.langsmith.trace_node(
                    node_name,
                    inputs={
                        "thread_id": thread_id,
                        "workflow_node": node_name,
                    },
                    outputs=node_output if isinstance(node_output, dict) else {"output": str(node_output)},
                )
                logger.debug(f"Node {node_name} completed: {node_output}")

        # Merge all results
        final_result: Dict[str, Any] = {"status": "completed"}
        for r in results:
            if isinstance(r, dict):
                final_result.update(r)

        return final_result

    async def ExecuteWorkflowStream(
        self,
        request: orchestration_pb2.WorkflowRequest,
        context: grpc.ServicerContext,
    ) -> AsyncIterator[orchestration_pb2.WorkflowUpdate]:
        """Execute a workflow with streaming updates."""
        logger.info(f"Executing workflow stream: {request.workflow_id}")

        try:
            parsed_request = parse_workflow_request(request)
        except WorkflowContractError as exc:
            yield orchestration_pb2.WorkflowUpdate(
                step_name="error",
                status="error",
                output=json.dumps({"error": str(exc)}),
                trace_id="",
            )
            return

        thread_id = parsed_request.thread_id
        self.registry.register(thread_id, request.workflow_id, status="streaming")

        try:
            # Get workflow
            workflow_builder = self._workflows.get(request.workflow_id)
            if not workflow_builder:
                yield orchestration_pb2.WorkflowUpdate(
                    step_name="error",
                    status="error",
                    output=json.dumps({"error": f"Unknown workflow: {request.workflow_id}"}),
                    trace_id="",
                )
                return

            # Build and stream workflow execution
            graph = workflow_builder()
            config = parsed_request.runnable_config()

            with self.langsmith.trace(
                request.workflow_id, thread_id, parsed_request.metadata
            ) as trace_id:
                async for event in graph.astream(parsed_request.input_data, config=config):
                    for node_name, node_output in event.items():
                        yield orchestration_pb2.WorkflowUpdate(
                            step_name=node_name,
                            status="in_progress",
                            output=json.dumps(node_output) if isinstance(node_output, dict) else str(node_output),
                            trace_id=trace_id or self.langsmith.get_current_trace_id(),
                        )
                        self.registry.update(
                            thread_id,
                            current_step=node_name,
                            increment_steps=True,
                        )
                        self.langsmith.trace_node(
                            node_name,
                            inputs={
                                "thread_id": thread_id,
                                "workflow_node": node_name,
                            },
                            outputs=node_output if isinstance(node_output, dict) else {"output": str(node_output)},
                        )

            # Final completion update
            yield orchestration_pb2.WorkflowUpdate(
                step_name="completed",
                status="completed",
                output="{}",
                trace_id=trace_id or self.langsmith.get_last_trace_id(),
            )
            self.registry.update(
                thread_id,
                status="completed",
                trace_id=trace_id or self.langsmith.get_last_trace_id(),
            )

        except Exception as e:
            logger.exception("Stream execution failed")
            yield orchestration_pb2.WorkflowUpdate(
                step_name="error",
                status="error",
                output=json.dumps({"error": str(e)}),
                trace_id=self.langsmith.get_current_trace_id() or self.langsmith.get_last_trace_id(),
            )
            self.registry.update(
                thread_id,
                status="error",
                error=str(e),
                trace_id=self.langsmith.get_current_trace_id() or self.langsmith.get_last_trace_id(),
            )

    async def GetWorkflowStatus(
        self,
        request: orchestration_pb2.StatusRequest,
        context: grpc.ServicerContext,
    ) -> orchestration_pb2.StatusResponse:
        """Check workflow status."""
        workflow = self.registry.get(request.thread_id)

        if not workflow:
            return orchestration_pb2.StatusResponse(
                status="not_found",
                current_step="",
                steps_completed=0,
            )

        return orchestration_pb2.StatusResponse(
            status=workflow.get("status", "unknown"),
            current_step=workflow.get("current_step", ""),
            steps_completed=workflow.get("steps_completed", 0),
        )


def create_server(port: int = 50051, max_workers: int = 10) -> grpc.Server:
    """Create and configure the gRPC server."""
    server = grpc.aio.server(
        futures.ThreadPoolExecutor(max_workers=max_workers),
        options=[
            ("grpc.max_send_message_length", 50 * 1024 * 1024),  # 50MB
            ("grpc.max_receive_message_length", 50 * 1024 * 1024),  # 50MB
        ],
    )

    orchestration_pb2_grpc.add_OrchestrationServiceServicer_to_server(
        OrchestrationServicer(), server
    )

    server.add_insecure_port(f"[::]:{port}")
    return server


async def serve(port: int = 50051) -> None:
    """Start the gRPC server."""
    server = create_server(port)
    await server.start()
    logger.info(f"Sidecar gRPC server started on port {port}")
    await server.wait_for_termination()


def main() -> None:
    """Main entry point."""
    parser = argparse.ArgumentParser(description="OpenRustClaw Python Sidecar")
    parser.add_argument("--port", type=int, default=int(os.getenv("SIDECAR_PORT", "50051")))
    parser.add_argument("--log-level", default=os.getenv("LOG_LEVEL", "INFO"))
    parser.add_argument("--langsmith-api-key", default=os.getenv("LANGSMITH_API_KEY"))
    parser.add_argument("--langsmith-project", default=os.getenv("LANGSMITH_PROJECT", "openrustclaw"))
    args = parser.parse_args()

    # Configure logging
    logging.basicConfig(
        level=getattr(logging, args.log_level.upper()),
        format="%(asctime)s - %(name)s - %(levelname)s - %(message)s",
    )

    # Set LangSmith environment variables
    if args.langsmith_api_key:
        os.environ["LANGSMITH_API_KEY"] = args.langsmith_api_key
    if args.langsmith_project:
        os.environ["LANGSMITH_PROJECT"] = args.langsmith_project

    logger.info(f"Starting OpenRustClaw sidecar on port {args.port}")

    try:
        asyncio.run(serve(args.port))
    except KeyboardInterrupt:
        logger.info("Shutting down sidecar")
    except Exception as e:
        logger.exception("Server error")
        raise


if __name__ == "__main__":
    main()
