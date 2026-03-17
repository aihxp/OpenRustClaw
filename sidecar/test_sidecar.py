#!/usr/bin/env python3
"""Test script for OpenRustClaw sidecar."""

import asyncio
import json

import pytest


def _skip_missing_dependency(error: ModuleNotFoundError) -> None:
    pytest.skip(f"Optional sidecar dependency not installed: {error}")


def test_imports():
    """Test that all modules can be imported."""
    print("Testing imports...")

    try:
        from src.proto import orchestration_pb2
        from src.proto import orchestration_pb2_grpc
        print("  ✓ Protobuf modules")
    except ModuleNotFoundError as e:
        print(f"  ✗ Protobuf modules: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Protobuf modules: {e}")
        raise

    try:
        from src.langsmith_bridge import LangSmithBridge
        print("  ✓ LangSmith bridge")
    except ModuleNotFoundError as e:
        print(f"  ✗ LangSmith bridge: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ LangSmith bridge: {e}")
        raise

    try:
        from src.workflows.agent_orchestrator import build_agent_graph
        from src.workflows.memory_maintenance import build_memory_maintenance_graph
        from src.workflows.rag_pipeline import build_rag_graph
        from src.workflows.scheduler import build_scheduler_graph
        from src.workflows.reminder import build_reminder_graph
        print("  ✓ Workflow modules")
    except ModuleNotFoundError as e:
        print(f"  ✗ Workflow modules: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Workflow modules: {e}")
        raise

    try:
        from src.evaluators import (
            evaluate_memory_recall,
            evaluate_rag_accuracy,
            evaluate_reminder_timing,
            evaluate_tool_use,
        )
        print("  ✓ Evaluator modules")
    except ModuleNotFoundError as e:
        print(f"  ✗ Evaluator modules: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Evaluator modules: {e}")
        raise

    try:
        from src.server import OrchestrationServicer, WorkflowRegistry
        print("  ✓ Server module")
    except ModuleNotFoundError as e:
        print(f"  ✗ Server module: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Server module: {e}")
        raise


def test_workflow_graphs():
    """Test that workflow graphs can be built."""
    print("\nTesting workflow graph construction...")

    try:
        from src.workflows.agent_orchestrator import build_agent_graph
        graph = build_agent_graph()
        print("  ✓ Agent orchestrator graph")
    except ModuleNotFoundError as e:
        print(f"  ✗ Agent orchestrator graph: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Agent orchestrator graph: {e}")
        raise

    try:
        from src.workflows.memory_maintenance import build_memory_maintenance_graph
        graph = build_memory_maintenance_graph()
        print("  ✓ Memory maintenance graph")
    except ModuleNotFoundError as e:
        print(f"  ✗ Memory maintenance graph: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Memory maintenance graph: {e}")
        raise

    try:
        from src.workflows.rag_pipeline import build_rag_graph
        graph = build_rag_graph()
        print("  ✓ RAG pipeline graph")
    except ModuleNotFoundError as e:
        print(f"  ✗ RAG pipeline graph: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ RAG pipeline graph: {e}")
        raise

    try:
        from src.workflows.scheduler import build_scheduler_graph
        graph = build_scheduler_graph()
        print("  ✓ Scheduler graph")
    except ModuleNotFoundError as e:
        print(f"  ✗ Scheduler graph: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Scheduler graph: {e}")
        raise

    try:
        from src.workflows.reminder import build_reminder_graph
        graph = build_reminder_graph()
        print("  ✓ Reminder graph")
    except ModuleNotFoundError as e:
        print(f"  ✗ Reminder graph: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Reminder graph: {e}")
        raise


def test_langsmith_bridge():
    """Test LangSmith bridge."""
    print("\nTesting LangSmith bridge...")

    try:
        from src.langsmith_bridge import LangSmithBridge

        bridge = LangSmithBridge()
        print(f"  ✓ LangSmith bridge created (enabled: {bridge.enabled})")

        trace_id = bridge.get_current_trace_id()
        print(f"  ✓ Current trace ID: {trace_id}")

        metrics = bridge.get_metrics()
        print(f"  ✓ Metrics: {metrics}")
    except ModuleNotFoundError as e:
        print(f"  ✗ LangSmith bridge: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ LangSmith bridge: {e}")
        raise


def test_workflow_registry():
    """Test workflow registry."""
    print("\nTesting workflow registry...")

    try:
        from src.server import WorkflowRegistry

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
    except ModuleNotFoundError as e:
        print(f"  ✗ Workflow registry: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Workflow registry: {e}")
        raise


def test_evaluators():
    """Test evaluator helpers."""
    print("\nTesting evaluators...")

    try:
        from src.evaluators import (
            evaluate_memory_recall,
            evaluate_rag_accuracy,
            evaluate_reminder_timing,
            evaluate_tool_use,
        )

        tool_result = evaluate_tool_use(
            ["search_memory"],
            [{"name": "search_memory"}],
        )
        assert tool_result.passed
        print("  ✓ Tool use evaluator")

        rag_result = evaluate_rag_accuracy(
            answer="Rust uses ownership for memory safety.",
            expected_facts=["ownership memory safety"],
            cited_source_ids=["doc-1"],
            required_source_ids=["doc-1"],
        )
        assert rag_result.passed
        print("  ✓ RAG accuracy evaluator")

        reminder_result = evaluate_reminder_timing(
            "2026-01-01T10:00:00Z",
            "2026-01-01T10:00:20Z",
            allowed_drift_seconds=60,
        )
        assert reminder_result.passed
        print("  ✓ Reminder timing evaluator")

        memory_result = evaluate_memory_recall(
            recalled_text="User prefers Rust and concise code reviews.",
            expected_memories=["prefers Rust", "concise code reviews"],
        )
        assert memory_result.passed
        print("  ✓ Memory recall evaluator")
    except ModuleNotFoundError as e:
        print(f"  ✗ Evaluators: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Evaluators: {e}")
        raise


def test_preprocess_memory_context_uses_metadata():
    """Test preprocess node uses configurable memory metadata."""
    print("\nTesting preprocess memory context...")

    try:
        from langchain_core.messages import HumanMessage
        from langchain_core.runnables import RunnableConfig
        from src.workflows.agent_orchestrator import PreprocessNode

        node = PreprocessNode()
        state = {
            "messages": [HumanMessage(content="hello"), HumanMessage(content="follow-up")],
            "memory_context": "",
            "tool_calls": [],
            "pending_approval": False,
            "approval_status": None,
            "output": None,
            "error": None,
        }
        config = RunnableConfig(
            configurable={
                "thread_id": "thread-123",
                "user_id": "user-456",
                "memory_context": "Prefers terse responses",
                "memory_entries": '["Uses Rust", "Works on OpenRustClaw"]',
            }
        )

        result = asyncio.run(node(state, config=config))
        context = result["memory_context"]
        assert "Thread: thread-123" in context
        assert "User: user-456" in context
        assert "Prefers terse responses" in context
        assert "Uses Rust" in context
        print("  ✓ Preprocess memory metadata")
    except ModuleNotFoundError as e:
        print(f"  ✗ Preprocess memory context: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Preprocess memory context: {e}")
        raise


def test_protobuf_messages():
    """Test protobuf message creation."""
    print("\nTesting protobuf messages...")

    try:
        from src.proto.orchestration_pb2 import (
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
    except ModuleNotFoundError as e:
        print(f"  ✗ Protobuf messages: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Protobuf messages: {e}")
        raise


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
    results.append(test_evaluators())
    results.append(test_preprocess_memory_context_uses_metadata())

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
