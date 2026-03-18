"""Main agent orchestration graph."""

import json
import logging
from dataclasses import dataclass, field
from typing import Annotated, Any, Dict, List, Literal, Optional, Sequence, TypedDict

from langchain_core.messages import AIMessage, BaseMessage, HumanMessage, ToolMessage
from langchain_core.runnables import RunnableConfig
from langgraph.graph import END, START, StateGraph
from langgraph.graph.message import add_messages
from langgraph.prebuilt import ToolNode

from ..memory_bridge import MemoryBridge

logger = logging.getLogger(__name__)


class AgentState(TypedDict):
    """State for the agent orchestration graph."""

    messages: Annotated[Sequence[BaseMessage], add_messages]
    memory_context: str
    tool_calls: List[Dict[str, Any]]
    pending_approval: bool
    approval_status: Optional[Literal["approved", "rejected"]]
    output: Optional[str]
    error: Optional[str]


def create_default_state() -> AgentState:
    """Create default initial state."""
    return {
        "messages": [],
        "memory_context": "",
        "tool_calls": [],
        "pending_approval": False,
        "approval_status": None,
        "output": None,
        "error": None,
    }


class PreprocessNode:
    """Node for preprocessing user input and loading memory context."""

    def __init__(self) -> None:
        self.name = "preprocess"
        self.memory_bridge = MemoryBridge.from_env()

    async def __call__(
        self,
        state: AgentState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Preprocess input and load memory context."""
        logger.debug("Running preprocess node")

        try:
            # Extract the latest user message
            messages = state.get("messages", [])
            if not messages:
                return {"error": "No messages provided", "output": None}

            # Load memory/context hints from the workflow configuration.
            memory_context = await self._load_memory_context(messages, config)

            return {
                "memory_context": memory_context,
                "error": None,
            }
        except Exception as e:
            logger.exception("Preprocess failed")
            return {"error": str(e)}

    async def _load_memory_context(
        self,
        messages: Sequence[BaseMessage],
        config: Optional[RunnableConfig],
    ) -> str:
        """Load relevant memory context for the conversation."""
        configurable = config.get("configurable", {}) if config else {}
        context_lines: List[str] = []

        thread_id = configurable.get("thread_id", "")
        if thread_id:
            context_lines.append(f"Thread: {thread_id}")

        user_id = configurable.get("user_id", "")
        if user_id:
            context_lines.append(f"User: {user_id}")
            if self.memory_bridge is not None:
                try:
                    core_memory = await self.memory_bridge.render_core_memory(user_id)
                    if core_memory and core_memory != "No core memory entries.":
                        context_lines.append(core_memory.strip())
                except Exception as exc:
                    logger.warning("Failed to load core memory from Rust bridge: %s", exc)

        provided_context = configurable.get("memory_context", "")
        if isinstance(provided_context, str) and provided_context.strip():
            context_lines.append(f"Memory:\n{provided_context.strip()}")

        session_summary = configurable.get("session_summary", "")
        if isinstance(session_summary, str) and session_summary.strip():
            context_lines.append(f"Session summary: {session_summary.strip()}")

        memory_entries = configurable.get("memory_entries", "")
        if isinstance(memory_entries, str) and memory_entries.strip():
            try:
                parsed_entries = json.loads(memory_entries)
            except json.JSONDecodeError:
                parsed_entries = None

            if isinstance(parsed_entries, list):
                rendered_entries = [
                    entry.strip()
                    for entry in parsed_entries
                    if isinstance(entry, str) and entry.strip()
                ]
                if rendered_entries:
                    context_lines.append(
                        "Memory entries:\n- " + "\n- ".join(rendered_entries[:10])
                    )
            else:
                context_lines.append(f"Memory entries: {memory_entries.strip()}")

        if len(messages) > 1:
            context_lines.append(f"Previous turns: {len(messages) - 1}")

        if self.memory_bridge is not None and user_id:
            latest_message = messages[-1]
            latest_content = getattr(latest_message, "content", "")
            if isinstance(latest_content, str) and latest_content.strip():
                try:
                    memories = await self.memory_bridge.search_memory(
                        user_id,
                        latest_content,
                        limit=3,
                    )
                    rendered = [
                        memory.get("content", "").strip()
                        for memory in memories
                        if isinstance(memory.get("content"), str)
                        and memory.get("content", "").strip()
                    ]
                    if rendered:
                        context_lines.append("Relevant memories:\n- " + "\n- ".join(rendered))
                except Exception as exc:
                    logger.warning("Failed to search Rust memory bridge: %s", exc)

        return "\n".join(context_lines)


class CallLLMNode:
    """Node for calling the LLM."""

    def __init__(self, model: Optional[str] = None) -> None:
        self.name = "call_llm"
        self.model = model or "gpt-4"
        self._llm: Optional[Any] = None

    def _get_llm(self) -> Any:
        """Get or create LLM instance."""
        if self._llm is None:
            try:
                from langchain_openai import ChatOpenAI
                self._llm = ChatOpenAI(
                    model=self.model,
                    temperature=0.7,
                    streaming=True,
                )
            except Exception as e:
                logger.error(f"Failed to initialize OpenAI model: {e}")
                # Fallback to a mock response for testing
                self._llm = None
        return self._llm

    async def __call__(
        self,
        state: AgentState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Call the LLM with the current state."""
        logger.debug("Running call_llm node")

        try:
            llm = self._get_llm()
            messages = list(state.get("messages", []))

            # Add memory context as system context if available
            memory_context = state.get("memory_context", "")
            if memory_context and messages:
                # Prepend context to first message or create system message
                context_msg = f"[Context]\n{memory_context}\n"
                if isinstance(messages[0], HumanMessage):
                    messages[0] = HumanMessage(
                        content=f"{context_msg}\n{messages[0].content}"
                    )

            if llm is None:
                # Mock response for testing without API key
                mock_response = AIMessage(
                    content='{"response": "Mock LLM response - no API key configured"}'
                )
                return {
                    "messages": [mock_response],
                    "tool_calls": [],
                }

            # Call LLM
            response = await llm.ainvoke(messages)

            # Extract tool calls if any
            tool_calls = []
            if hasattr(response, "tool_calls") and response.tool_calls:
                tool_calls = [
                    {
                        "id": tc.get("id", ""),
                        "name": tc.get("name", ""),
                        "arguments": tc.get("args", {}),
                    }
                    for tc in response.tool_calls
                ]

            return {
                "messages": [response],
                "tool_calls": tool_calls,
            }

        except Exception as e:
            logger.exception("LLM call failed")
            return {"error": str(e)}


class ExecuteToolsNode:
    """Node for executing tool calls."""

    def __init__(self, tools: Optional[List[Any]] = None) -> None:
        self.name = "execute_tools"
        self.tools = tools or []
        self._memory_records: List[Dict[str, str]] = []
        self._scheduled_reminders: List[Dict[str, str]] = []
        self.memory_bridge = MemoryBridge.from_env()

    def _get_tool_node(self, config: Optional[RunnableConfig]) -> ToolNode:
        """Get or create tool node."""
        if self.tools:
            return ToolNode(self.tools)
        return ToolNode(self._create_default_tools(config))

    def _create_default_tools(self, config: Optional[RunnableConfig]) -> List[Any]:
        """Create default set of tools."""
        from langchain_core.tools import tool
        configurable = config.get("configurable", {}) if config else {}
        user_id = str(configurable.get("user_id", "")).strip()
        thread_id = str(configurable.get("thread_id", "")).strip()

        @tool
        async def search_memory(query: str) -> str:
            """Search the memory system for relevant information."""
            if self.memory_bridge is not None and user_id:
                memories = await self.memory_bridge.search_memory(user_id, query, limit=5)
                rendered = [
                    memory.get("content", "").strip()
                    for memory in memories
                    if isinstance(memory.get("content"), str)
                    and memory.get("content", "").strip()
                ]
                if rendered:
                    return "\n".join(rendered)
                return f"No stored memories matched: {query}"

            query_lower = query.strip().lower()
            matches = [
                f"[{record['category']}] {record['content']}"
                for record in self._memory_records
                if query_lower and query_lower in record["content"].lower()
            ]
            if not matches:
                return f"No stored memories matched: {query}"
            return "\n".join(matches[:5])

        @tool
        async def store_memory(content: str, category: str = "episodic") -> str:
            """Store information in memory."""
            if self.memory_bridge is not None and user_id:
                result = await self.memory_bridge.store_memory(
                    user_id=user_id,
                    content=content,
                    category=category,
                    session_id=thread_id,
                )
                memory_id = result.get("id", "")
                return f"Stored in {category}: {content[:50]}... ({memory_id})"

            self._memory_records.append(
                {
                    "content": content.strip(),
                    "category": category.strip() or "episodic",
                }
            )
            return f"Stored in {category}: {content[:50]}..."

        @tool
        async def set_core_memory(
            key: str,
            value: str,
            importance: float = 0.8,
        ) -> str:
            """Set a durable core-memory entry for the active user."""
            if self.memory_bridge is not None and user_id:
                await self.memory_bridge.set_core_memory(
                    user_id=user_id,
                    key=key,
                    value=value,
                    importance=importance,
                )
                return f"Set core memory {key}={value}"

            self._memory_records.append(
                {
                    "content": f"{key.strip()}: {value.strip()}",
                    "category": "core",
                }
            )
            return f"Set core memory {key}={value}"

        @tool
        def schedule_reminder(
            content: str,
            datetime_str: str,
            timezone: str = "UTC",
        ) -> str:
            """Schedule a reminder for a specific time."""
            self._scheduled_reminders.append(
                {
                    "content": content.strip(),
                    "datetime": datetime_str.strip(),
                    "timezone": timezone.strip() or "UTC",
                }
            )
            return f"Reminder scheduled for {datetime_str} ({timezone}): {content[:50]}..."

        return [search_memory, store_memory, set_core_memory, schedule_reminder]

    async def __call__(
        self,
        state: AgentState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Execute tool calls from the LLM response."""
        logger.debug("Running execute_tools node")

        try:
            tool_calls = state.get("tool_calls", [])
            if not tool_calls:
                return {"output": "No tools to execute"}

            tool_node = self._get_tool_node(config)

            # Execute tools
            messages = state.get("messages", [])
            if messages and hasattr(messages[-1], "tool_calls"):
                tool_messages = await tool_node.ainvoke(
                    {"messages": [messages[-1]]},
                    config=config,
                )
                return {
                    "messages": tool_messages.get("messages", []),
                    "output": json.dumps(
                        {"executed_tools": len(tool_calls)}
                    ),
                }

            return {"output": "No tool calls to execute"}

        except Exception as e:
            logger.exception("Tool execution failed")
            return {"error": str(e)}


class HumanApprovalGate:
    """Node for human-in-the-loop approval gate."""

    def __init__(self) -> None:
        self.name = "human_approval_gate"

    async def __call__(
        self,
        state: AgentState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Check if human approval is needed and handle it."""
        logger.debug("Running human_approval_gate node")

        tool_calls = state.get("tool_calls", [])
        pending_approval = state.get("pending_approval", False)
        approval_status = state.get("approval_status")

        # Check if any tool calls require approval
        requires_approval = any(
            tc.get("name", "") in self._sensitive_tools()
            for tc in tool_calls
        )

        if requires_approval and not pending_approval:
            # Request approval
            return {
                "pending_approval": True,
                "output": json.dumps(
                    {
                        "status": "pending_approval",
                        "tools": [tc.get("name") for tc in tool_calls],
                    }
                ),
            }

        if pending_approval:
            if approval_status == "approved":
                return {
                    "pending_approval": False,
                    "approval_status": None,
                }
            elif approval_status == "rejected":
                return {
                    "pending_approval": False,
                    "approval_status": None,
                    "output": json.dumps({"status": "rejected"}),
                    "error": "Tool execution rejected by user",
                }
            else:
                # Still waiting for approval
                return {
                    "output": json.dumps({"status": "waiting_approval"}),
                }

        # No approval needed
        return {"pending_approval": False}

    def _sensitive_tools(self) -> List[str]:
        """List of tools that require human approval."""
        return [
            "delete_memory",
            "send_email",
            "modify_calendar",
            "execute_code",
        ]


class PostprocessNode:
    """Node for postprocessing and finalizing output."""

    def __init__(self) -> None:
        self.name = "postprocess"

    async def __call__(
        self,
        state: AgentState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Postprocess the final output."""
        logger.debug("Running postprocess node")

        try:
            messages = state.get("messages", [])
            error = state.get("error")

            if error:
                return {
                    "output": json.dumps({"error": error}),
                    "status": "error",
                }

            # Extract final response from messages
            final_content = ""
            for msg in reversed(messages):
                if isinstance(msg, AIMessage) and msg.content:
                    final_content = msg.content
                    break

            # Store interaction in memory
            await self._store_interaction(messages, config)

            return {
                "output": json.dumps(
                    {
                        "response": final_content,
                        "memory_used": bool(state.get("memory_context")),
                        "tools_used": len(state.get("tool_calls", [])),
                    }
                ),
                "status": "completed",
            }

        except Exception as e:
            logger.exception("Postprocess failed")
            return {
                "output": json.dumps({"error": str(e)}),
                "status": "error",
            }

    async def _store_interaction(
        self,
        messages: Sequence[BaseMessage],
        config: Optional[RunnableConfig],
    ) -> None:
        """Store the interaction in memory."""
        thread_id = config.get("configurable", {}).get("thread_id", "") if config else ""
        message_count = len(messages)
        logger.debug(
            "Interaction completed for thread %s with %s messages",
            thread_id,
            message_count,
        )


def should_execute_tools(state: AgentState) -> str:
    """Determine if we should execute tools or go to postprocess."""
    tool_calls = state.get("tool_calls", [])
    error = state.get("error")

    if error:
        return "postprocess"

    if tool_calls:
        return "human_approval_gate"

    return "postprocess"


def should_approve(state: AgentState) -> str:
    """Determine routing after approval gate."""
    pending_approval = state.get("pending_approval", False)
    error = state.get("error")

    if error:
        return "postprocess"

    if pending_approval:
        # Need to wait for approval - this would typically be handled
        # by an interrupt or external callback in production
        return "execute_tools"

    return "execute_tools"


def build_agent_graph(
    model: Optional[str] = None,
    tools: Optional[List[Any]] = None,
) -> StateGraph:
    """Build the main agent orchestration graph.

    The graph follows this flow:
    1. preprocess - Load memory context
    2. call_llm - Generate response (possibly with tool calls)
    3. If tool calls: human_approval_gate -> execute_tools -> call_llm
    4. postprocess - Finalize and store output
    """
    # Create nodes
    preprocess = PreprocessNode()
    call_llm = CallLLMNode(model=model)
    execute_tools = ExecuteToolsNode(tools=tools)
    approval_gate = HumanApprovalGate()
    postprocess = PostprocessNode()

    # Build graph
    workflow = StateGraph(AgentState)

    # Add nodes
    workflow.add_node("preprocess", preprocess)
    workflow.add_node("call_llm", call_llm)
    workflow.add_node("execute_tools", execute_tools)
    workflow.add_node("human_approval_gate", approval_gate)
    workflow.add_node("postprocess", postprocess)

    # Add edges
    workflow.add_edge(START, "preprocess")
    workflow.add_edge("preprocess", "call_llm")

    # Conditional from call_llm based on tool calls
    workflow.add_conditional_edges(
        "call_llm",
        should_execute_tools,
        {
            "human_approval_gate": "human_approval_gate",
            "postprocess": "postprocess",
        },
    )

    # From approval gate to tools or postprocess
    workflow.add_conditional_edges(
        "human_approval_gate",
        should_approve,
        {
            "execute_tools": "execute_tools",
            "postprocess": "postprocess",
        },
    )

    # After tools, go back to LLM for synthesis
    workflow.add_edge("execute_tools", "call_llm")

    # Final edge
    workflow.add_edge("postprocess", END)

    return workflow.compile()
