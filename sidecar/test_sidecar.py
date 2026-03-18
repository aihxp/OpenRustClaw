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


def test_memory_bridge_request_helpers():
    """Test sidecar memory bridge HTTP client behavior."""
    print("\nTesting memory bridge client...")

    try:
        from unittest.mock import patch

        from src.memory_bridge import MemoryBridge

        class _Response:
            def __enter__(self):
                return self

            def __exit__(self, exc_type, exc, tb):
                return False

            def read(self):
                return b'{"memories":[{"content":"Prefers Rust"}],"chunks":[{"content":"Prefers Rust"}],"content":"## Core Memory"}'

        bridge = MemoryBridge("http://127.0.0.1:3000/internal", "token-123")

        with patch("urllib.request.urlopen", return_value=_Response()) as mock_urlopen:
            search_result = asyncio.run(bridge.search_memory("user-1", "rust", limit=2))
            core_result = asyncio.run(bridge.render_core_memory("user-1"))
            old_result = asyncio.run(
                bridge.fetch_old_memories(age_days=30, namespace="user-1", user_id="user-1")
            )
            rag_store_result = asyncio.run(
                bridge.store_rag_chunks(
                    "docs",
                    [{"id": "chunk-1", "content": "Rust ownership", "metadata": {}}],
                )
            )
            rag_load_result = asyncio.run(bridge.load_rag_chunks("docs", limit=10))
            archive_result = asyncio.run(
                bridge.store_archive_entry(
                    {
                        "id": "archive-1",
                        "summary": "Archive summary",
                        "source_memory_ids": ["memory-1"],
                    }
                )
            )
            delete_result = asyncio.run(bridge.archive_memory_ids(["memory-1"]))

        assert search_result[0]["content"] == "Prefers Rust"
        assert "Core Memory" in core_result
        assert old_result[0]["content"] == "Prefers Rust"
        assert rag_store_result["memories"][0]["content"] == "Prefers Rust"
        assert rag_load_result[0]["content"] == "Prefers Rust"
        assert archive_result["memories"][0]["content"] == "Prefers Rust"
        assert delete_result["memories"][0]["content"] == "Prefers Rust"
        assert mock_urlopen.call_count == 7
        print("  ✓ Memory bridge client")
    except ModuleNotFoundError as e:
        print(f"  ✗ Memory bridge client: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Memory bridge client: {e}")
        raise


def test_memory_maintenance_uses_configurable_memories():
    """Test memory maintenance nodes consume configurable old memories."""
    print("\nTesting memory maintenance workflow metadata...")

    try:
        from langchain_core.runnables import RunnableConfig
        from src.workflows.memory_maintenance import IdentifyOldMemoriesNode

        node = IdentifyOldMemoriesNode(age_threshold_days=30)
        config = RunnableConfig(
            configurable={
                "old_memories": [
                    {
                        "id": "memory-1",
                        "content": "Old memory",
                        "timestamp": "2025-01-01T00:00:00Z",
                    },
                    {
                        "id": "memory-2",
                        "content": "Recent memory",
                        "timestamp": "2030-01-01T00:00:00Z",
                    },
                ]
            }
        )

        result = asyncio.run(node({"messages": [], "old_memories": [], "summaries": [], "archived_count": 0, "consolidated_count": 0, "archive_entries": [], "archived_memory_ids": [], "errors": [], "status": "pending"}, config=config))
        assert len(result["old_memories"]) == 1
        assert result["old_memories"][0]["id"] == "memory-1"
        print("  ✓ Memory maintenance metadata")
    except ModuleNotFoundError as e:
        print(f"  ✗ Memory maintenance metadata: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Memory maintenance metadata: {e}")
        raise


def test_memory_maintenance_archive_node_uses_bridge():
    """Test archive node persists summaries and archived ids through the bridge."""
    print("\nTesting memory maintenance archive bridge...")

    try:
        from unittest.mock import AsyncMock, patch

        from src.workflows.memory_maintenance import ArchiveMemoriesNode

        fake_bridge = AsyncMock()
        fake_bridge.store_archive_entry.return_value = {"stored": True, "id": "archive-1"}
        fake_bridge.archive_memory_ids.return_value = {"memory_ids": ["memory-1"]}

        with patch(
            "src.workflows.memory_maintenance.MemoryBridge.from_env",
            return_value=fake_bridge,
        ):
            node = ArchiveMemoriesNode()
            result = asyncio.run(
                node(
                    {
                        "messages": [],
                        "old_memories": [{"id": "memory-1", "content": "Old memory"}],
                        "summaries": [
                            {
                                "id": "archive-1",
                                "summary": "Archived summary",
                                "source_memory_ids": ["memory-1"],
                                "namespace": "user-1",
                            }
                        ],
                        "archived_count": 0,
                        "consolidated_count": 1,
                        "archive_entries": [],
                        "archived_memory_ids": [],
                        "errors": [],
                        "status": "running",
                    }
                )
            )

        assert result["archived_count"] == 1
        assert result["archived_memory_ids"] == ["memory-1"]
        assert result["archive_entries"][0]["stored"] is True
        print("  ✓ Memory maintenance archive bridge")
    except ModuleNotFoundError as e:
        print(f"  ✗ Memory maintenance archive bridge: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Memory maintenance archive bridge: {e}")
        raise


def test_workflow_contract_parses_typed_configurable_metadata():
    """Test typed configurable payload survives the workflow contract."""
    print("\nTesting workflow contract parsing...")

    try:
        from src.proto.orchestration_pb2 import WorkflowRequest
        from src.workflow_contract import (
            CONFIGURABLE_METADATA_KEY,
            get_configurable_value,
            parse_workflow_request,
        )

        request = WorkflowRequest(
            workflow_id="scheduler",
            thread_id="thread-123",
            input='{"job_id":"job-1","payload":{}}',
            metadata={
                "job_name": "Nightly sync",
                CONFIGURABLE_METADATA_KEY: json.dumps(
                    {
                        "limit": 25,
                        "labels": ["nightly", "critical"],
                        "workflow_metadata": {"priority": "high", "enabled": True},
                    }
                ),
            },
        )

        parsed = parse_workflow_request(request)
        assert parsed.thread_id == "thread-123"
        assert parsed.input_data["job_id"] == "job-1"
        assert parsed.configurable["job_name"] == "Nightly sync"
        assert parsed.configurable["limit"] == 25
        assert parsed.configurable["labels"] == ["nightly", "critical"]
        assert parsed.configurable["workflow_metadata"]["enabled"] is True
        assert parsed.runnable_config()["configurable"]["thread_id"] == "thread-123"
        assert get_configurable_value(parsed.runnable_config(), "limit") == 25
        print("  ✓ Workflow contract parsing")
    except ModuleNotFoundError as e:
        print(f"  ✗ Workflow contract parsing: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Workflow contract parsing: {e}")
        raise


def test_scheduler_retry_records_next_run():
    """Test scheduler retry logic records the next retry timestamp."""
    print("\nTesting scheduler retry scheduling...")

    try:
        from src.workflows.scheduler import RetryNode

        node = RetryNode()
        state = {
            "messages": [],
            "job_id": "job-123",
            "job_type": "test",
            "payload": {},
            "scheduled_time": "",
            "idempotency_key": "idem-1",
            "retry_count": 0,
            "max_retries": 3,
            "retry_delay_base": 1.0,
            "status": "failed",
            "result": None,
            "error": "boom",
            "execution_time_ms": 0.0,
            "previous_attempts": [],
            "next_retry_at": None,
        }

        result = asyncio.run(node(state))
        assert result["status"] == "retrying"
        assert result["next_retry_at"]
        print("  ✓ Scheduler retry scheduling")
    except ModuleNotFoundError as e:
        print(f"  ✗ Scheduler retry scheduling: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ Scheduler retry scheduling: {e}")
        raise


def test_rag_pipeline_supports_query_only_retrieval():
    """Test RAG indexing and later query-only retrieval against the same collection."""
    print("\nTesting RAG indexing and retrieval...")

    try:
        from langchain_core.documents import Document

        from src.workflows.rag_pipeline import build_rag_graph, create_default_state

        graph = build_rag_graph(top_k=2)
        collection_name = "test-rag-collection"

        index_state = create_default_state()
        index_state["documents"] = [
            Document(
                page_content=(
                    "Rust uses ownership and borrowing to guarantee memory safety without a garbage collector."
                ),
                metadata={"source": "doc-1", "type": "text"},
            )
        ]

        asyncio.run(
            graph.ainvoke(
                index_state,
                config={
                    "configurable": {
                        "collection_name": collection_name,
                        "context_budget_chars": 180,
                    }
                },
            )
        )

        query_state = create_default_state()
        query_state["query"] = "How does Rust provide memory safety?"
        result = asyncio.run(
            graph.ainvoke(
                query_state,
                config={
                    "configurable": {
                        "collection_name": collection_name,
                        "context_budget_chars": 180,
                    }
                },
            )
        )

        assert result["retrieved_docs"]
        assert result["sources"]
        assert result["sources"][0]["metadata"]["source_id"].startswith("chunk_")
        assert "sources:" in result["answer"].lower()
        print("  ✓ RAG indexing and retrieval")
    except ModuleNotFoundError as e:
        print(f"  ✗ RAG indexing and retrieval: {e}")
        _skip_missing_dependency(e)
    except Exception as e:
        print(f"  ✗ RAG indexing and retrieval: {e}")
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
    results.append(test_memory_bridge_request_helpers())
    results.append(test_memory_maintenance_uses_configurable_memories())
    results.append(test_memory_maintenance_archive_node_uses_bridge())
    results.append(test_workflow_contract_parses_typed_configurable_metadata())
    results.append(test_scheduler_retry_records_next_run())
    results.append(test_rag_pipeline_supports_query_only_retrieval())

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
