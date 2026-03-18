"""Typed helpers for the Rust-sidecar workflow contract."""

from __future__ import annotations

import json
from dataclasses import dataclass
from typing import Any, Dict, Optional

from .proto import orchestration_pb2

CONFIGURABLE_METADATA_KEY = "__openrustclaw_configurable"


class WorkflowContractError(ValueError):
    """Raised when a workflow request cannot be parsed."""


@dataclass
class ParsedWorkflowRequest:
    """Normalized view of a Rust-sidecar workflow request."""

    workflow_id: str
    thread_id: str
    input_data: Dict[str, Any]
    metadata: Dict[str, str]
    configurable: Dict[str, Any]

    def runnable_config(self) -> Dict[str, Dict[str, Any]]:
        """Build the RunnableConfig-compatible wrapper."""
        return {"configurable": self.configurable}


def parse_workflow_request(
    request: orchestration_pb2.WorkflowRequest,
) -> ParsedWorkflowRequest:
    """Parse the protobuf request into typed input and configurable state."""
    thread_id = request.thread_id.strip()
    if not thread_id:
        raise WorkflowContractError("thread_id is required")

    try:
        input_data = json.loads(request.input) if request.input else {}
    except json.JSONDecodeError as exc:
        raise WorkflowContractError(f"Invalid input JSON: {exc}") from exc

    if not isinstance(input_data, dict):
        raise WorkflowContractError("Workflow input must decode to a JSON object")

    metadata = dict(request.metadata)
    configurable = {
        key: value
        for key, value in metadata.items()
        if key != CONFIGURABLE_METADATA_KEY
    }

    encoded_configurable = metadata.get(CONFIGURABLE_METADATA_KEY)
    if encoded_configurable:
        try:
            decoded = json.loads(encoded_configurable)
        except json.JSONDecodeError as exc:
            raise WorkflowContractError(
                f"Invalid configurable metadata JSON: {exc}"
            ) from exc
        if not isinstance(decoded, dict):
            raise WorkflowContractError(
                "Configurable metadata must decode to a JSON object"
            )
        configurable.update(decoded)

    configurable["thread_id"] = thread_id

    return ParsedWorkflowRequest(
        workflow_id=request.workflow_id,
        thread_id=thread_id,
        input_data=input_data,
        metadata=metadata,
        configurable=configurable,
    )


def get_configurable_value(
    config: Optional[Dict[str, Any]],
    key: str,
    default: Any = None,
) -> Any:
    """Load a configurable value, decoding legacy JSON strings when needed."""
    configurable = config.get("configurable", {}) if config else {}
    value = configurable.get(key, default)
    if isinstance(value, str):
        stripped = value.strip()
        if stripped and stripped[0] in "[{":
            try:
                return json.loads(stripped)
            except json.JSONDecodeError:
                return value
    return value
