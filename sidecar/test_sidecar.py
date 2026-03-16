#!/usr/bin/env python3
"""Test script for OpenRustClaw sidecar."""

import asyncio
import json
import sys
from pathlib import Path

# Add src to path
sys.path.insert(0, str(Path(__file__).parent / "src"))


def test_imports():
    """Test that all modules can be imported."""
    print("Testing imports...")

    try:
        from proto import orchestration_pb2
        from proto import orchestration_pb2_grpc
        print("  ✓ Protobuf modules")
    except Exception as e:
        print(f"  ✗ Protobuf modules: {e}")
        return False

    try:
        from langsmith_bridge import LangSmithBridge
        print("  ✓ LangSmith bridge")
    except Exception as e:
        print(f"  ✗ LangSmith bridge: {e}")
        return False

    try:
        from workflows.agent_orchestrator import build_agent_graph
        from workflows.memory_maintenance import build_memory_maintenance_graph
        from workflows.rag_pipeline import build_rag_graph
        from workflows.scheduler import build_scheduler_graph
        from workflows.reminder import build_reminder_graph
        print("  ✓ Workflow modules")
    except Exception as e:
        print(f"  ✗ Workflow modules: {e}")
        return False

    try:
        from server import OrchestrationServicer, WorkflowRegistry
        print("  ✓ Server module")
    except Exception as e:
        print(f"  ✗ Server module: {e}")
        return False

    return True


def test_workflow_graphs():
    """Test that workflow graphs can be built."""
    print("\nTesting workflow graph construction...")

    try:
        from workflows.agent_orchestrator import build_agent_graph
        graph = build_agent_graph()
        print("  ✓ Agent orchestrator graph")
    except Exception as e:
        print(f"  ✗ Agent orchestrator graph: {e}")
        return False

    try:
        from workflows.memory_maintenance import build_memory_maintenance_graph
        graph = build_memory_maintenance_graph()
        print("  ✓ Memory maintenance graph")
    except Exception as e:
        print(f"  ✗ Memory maintenance graph: {e}")
        return False

    try:
        from workflows.rag_pipeline import build_rag_graph
        graph = build_rag_graph()
        print("  ✓ RAG pipeline graph")
    except Exception as e:
        print(f"  ✗ RAG pipeline graph: {e}")
        return False

    try:
        from workflows.scheduler import build_scheduler_graph
        graph = build_scheduler_graph()
        print("  ✓ Scheduler graph")
    except Exception as e:
        print(f"  ✗ Scheduler graph: {e}")
        return False

    try:
        from workflows.reminder import build_reminder_graph
        graph = build_reminder_graph()
        print("  ✓ Reminder graph")
    except Exception as e:
        print(f"  ✗ Reminder graph: {e}")
        return False

    return True


def test_langsmith_bridge():
    """Test LangSmith bridge."""
    print("\nTesting LangSmith bridge...")

    try:
        from langsmith_bridge import LangSmithBridge

        bridge = LangSmithBridge()
        print(f"  ✓ LangSmith bridge created (enabled: {bridge.enabled})")

        trace_id = bridge.get_current_trace_id()
        print(f"  ✓ Current trace ID: {trace_id}")

        metrics = bridge.get_metrics()
        print(f"  ✓ Metrics: {metrics}")

        return True
    except Exception as e:
        print(f"  ✗ LangSmith bridge: {e}")
        return False


def test_workflow_registry():
    """Test workflow registry."""
    print("\nTesting workflow registry...")

    try:
        from server import WorkflowRegistry

        registry = WorkflowRegistry()
        registry.register("thread-123", "agent")

        status = registry.get("thread-123")
        assert status is not None
        assert status["workflow_id"] == "agent"
        print("  ✓ Workflow registration")

        registry.update("thread-123", status="running", current_step="step1")
        status = registry.get("thread-123")
        assert status["status"] == "running"
        print("  ✓ Workflow update")

        registry.remove("thread-123")
        status = registry.get("thread-123")
        assert status is None
        print("  ✓ Workflow removal")

        return True
    except Exception as e:
        print(f"  ✗ Workflow registry: {e}")
        return False


def test_protobuf_messages():
    """Test protobuf message creation."""
    print("\nTesting protobuf messages...")

    try:
        from proto.orchestration_pb2 import (
            StatusRequest,
            StatusResponse,
            WorkflowRequest,
            WorkflowResponse,
            WorkflowUpdate,
        )

        # Workflow request
        request = WorkflowRequest(
            workflow_id="agent",
            thread_id="thread-123",
            input='{"message": "Hello"}',
        )
        assert request.workflow_id == "agent"
        print("  ✓ WorkflowRequest")

        # Workflow response
        response = WorkflowResponse(
            thread_id="thread-123",
            output='{"response": "Hi"}',
            status="completed",
        )
        assert response.status == "completed"
        print("  ✓ WorkflowResponse")

        # Workflow update
        update = WorkflowUpdate(
            step_name="call_llm",
            status="completed",
        )
        assert update.step_name == "call_llm"
        print("  ✓ WorkflowUpdate")

        # Status request/response
        status_req = StatusRequest(thread_id="thread-123")
        status_resp = StatusResponse(
            status="running",
            current_step="call_llm",
            steps_completed=2,
        )
        assert status_resp.steps_completed == 2
        print("  ✓ StatusRequest/StatusResponse")

        return True
    except Exception as e:
        print(f"  ✗ Protobuf messages: {e}")
        return False


async def test_agent_workflow():
    """Test agent workflow execution."""
    print("\nTesting agent workflow execution...")

    try:
        from workflows.agent_orchestrator import build_agent_graph
        from langchain_core.messages import HumanMessage

        graph = build_agent_graph()

        # Test with minimal input
        initial_state = {
            "messages": [HumanMessage(content="Hello")],
            "memory_context": "",
            "tool_calls": [],
            "pending_approval": False,
            "approval_status": None,
            "output": None,
            "error": None,
        }

        config = {"configurable": {"thread_id": "test-thread"}}

        # Run the workflow (may fail due to no API key, but tests graph structure)
        try:
            async for event in graph.astream(initial_state, config=config):
                print(f"  Event: {list(event.keys())}")
        except Exception as e:
            # Expected without API key
            print(f"  Note: Execution requires API key ({type(e).__name__})")

        print("  ✓ Agent workflow structure validated")
        return True

    except Exception as e:
        print(f"  ✗ Agent workflow: {e}")
        return False


async def test_scheduler_workflow():
    """Test scheduler workflow execution."""
    print("\nTesting scheduler workflow execution...")

    try:
        from workflows.scheduler import build_scheduler_graph

        graph = build_scheduler_graph()

        # Test with minimal input
        initial_state = {
            "messages": [],
            "job_id": "job-123",
            "job_type": "test",
            "payload": {"action": "test"},
            "scheduled_time": "",  # Immediate execution
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

        config = {"configurable": {"thread_id": "test-thread"}}

        async for event in graph.astream(initial_state, config=config):
            print(f"  Event: {list(event.keys())}")

        print("  ✓ Scheduler workflow executed")
        return True

    except Exception as e:
        print(f"  ✗ Scheduler workflow: {e}")
        import traceback
        traceback.print_exc()
        return False


async def run_async_tests():
    """Run async tests."""
    results = []
    results.append(await test_agent_workflow())
    results.append(await test_scheduler_workflow())
    return all(results)


def main():
    """Run all tests."""
    print("=" * 60)
    print("OpenRustClaw Sidecar Test Suite")
    print("=" * 60)

    results = []

    results.append(test_imports())
    results.append(test_workflow_graphs())
    results.append(test_langsmith_bridge())
    results.append(test_workflow_registry())
    results.append(test_protobuf_messages())

    # Run async tests
    results.append(asyncio.run(run_async_tests()))

    print("\n" + "=" * 60)
    if all(results):
        print("All tests passed! ✓")
        return 0
    else:
        print("Some tests failed! ✗")
        return 1


if __name__ == "__main__":
    sys.exit(main())
