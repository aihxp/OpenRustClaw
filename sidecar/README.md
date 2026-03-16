# OpenRustClaw Python Sidecar

This Python sidecar provides LangGraph workflow orchestration for the OpenRustClaw system via gRPC.

## Features

- **LangGraph Workflows**: Full state graph implementations for:
  - Agent orchestration with tool calling
  - Memory maintenance and consolidation
  - RAG pipeline for document processing
  - Scheduled job execution with retry logic
  - Reminder delivery with timezone support

- **gRPC Server**: High-performance RPC interface for Rust integration
  - Unary workflow execution
  - Streaming workflow updates
  - Status querying

- **LangSmith Integration**: Automatic tracing and monitoring
  - Trace export
  - Metrics collection
  - Evaluation logging

## Installation

```bash
# Create virtual environment
python -m venv .venv
source .venv/bin/activate  # or .venv\Scripts\activate on Windows

# Install dependencies
pip install -e ".[dev]"
```

## Quick Start

```bash
# Start the sidecar server
python -m src.server --port 50051

# Or use the installed command
openrustclaw-sidecar --port 50051
```

## Configuration

Environment variables:

- `SIDECAR_PORT`: Server port (default: 50051)
- `LOG_LEVEL`: Logging level (default: INFO)
- `LANGSMITH_API_KEY`: LangSmith API key for tracing
- `LANGSMITH_PROJECT`: LangSmith project name (default: openrustclaw)
- `OPENAI_API_KEY`: OpenAI API key for LLM features

## Workflows

### Agent Orchestration

```python
from workflows.agent_orchestrator import build_agent_graph

graph = build_agent_graph(model="gpt-4")
result = await graph.ainvoke({
    "messages": [HumanMessage(content="Hello")],
    "memory_context": "",
    "tool_calls": [],
    "pending_approval": False,
})
```

### Memory Maintenance

```python
from workflows.memory_maintenance import build_memory_maintenance_graph

graph = build_memory_maintenance_graph(age_threshold_days=30)
result = await graph.ainvoke({
    "old_memories": [...],
    "summaries": [],
})
```

### RAG Pipeline

```python
from workflows.rag_pipeline import build_rag_graph

graph = build_rag_graph(chunk_size=1000, top_k=5)
result = await graph.ainvoke({
    "documents": [Document(...)],
    "query": "What is...?",
})
```

### Scheduler

```python
from workflows.scheduler import build_scheduler_graph

graph = build_scheduler_graph(max_retries=3)
result = await graph.ainvoke({
    "job_id": "job-123",
    "job_type": "cleanup",
    "payload": {"action": "cleanup"},
})
```

### Reminder

```python
from workflows.reminder import build_reminder_graph

graph = build_reminder_graph()
result = await graph.ainvoke({
    "reminder_id": "rem-123",
    "content": "Meeting at 3pm",
    "scheduled_time": "2024-12-25T15:00:00",
    "timezone": "America/New_York",
})
```

## gRPC Protocol

The sidecar implements the `OrchestrationService`:

```protobuf
service OrchestrationService {
    rpc ExecuteWorkflow(WorkflowRequest) returns (WorkflowResponse);
    rpc ExecuteWorkflowStream(WorkflowRequest) returns (stream WorkflowUpdate);
    rpc GetWorkflowStatus(StatusRequest) returns (StatusResponse);
}
```

## Development

```bash
# Generate protobuf code
python generate_proto.py

# Run tests
python test_sidecar.py

# Type checking
mypy src/

# Linting
ruff check src/
```

## Project Structure

```
sidecar/
├── src/
│   ├── server.py              # gRPC server implementation
│   ├── langsmith_bridge.py    # LangSmith integration
│   ├── proto/                 # Generated protobuf code
│   │   ├── orchestration_pb2.py
│   │   └── orchestration_pb2_grpc.py
│   ├── workflows/             # LangGraph workflows
│   │   ├── agent_orchestrator.py
│   │   ├── memory_maintenance.py
│   │   ├── rag_pipeline.py
│   │   ├── scheduler.py
│   │   └── reminder.py
│   └── evaluators/            # Evaluation modules
├── tests/                     # Test files
├── pyproject.toml             # Project configuration
├── generate_proto.py          # Protobuf generation script
└── test_sidecar.py            # Test suite
```

## License

MIT License - See LICENSE file for details.
