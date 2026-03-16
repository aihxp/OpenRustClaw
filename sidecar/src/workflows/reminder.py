"""Reminder workflow for scheduled reminders with timezone handling."""

import json
import logging
from datetime import datetime, timedelta
from typing import Annotated, Any, Dict, List, Literal, Optional, Sequence, TypedDict

import pytz
from dateutil import parser as date_parser
from langchain_core.messages import AIMessage, BaseMessage, HumanMessage, SystemMessage
from langchain_core.runnables import RunnableConfig
from langgraph.graph import END, START, StateGraph
from langgraph.graph.message import add_messages

logger = logging.getLogger(__name__)


class ReminderState(TypedDict):
    """State for the reminder workflow."""

    messages: Annotated[Sequence[BaseMessage], add_messages]
    reminder_id: str
    content: str
    scheduled_time: str  # ISO format
    timezone: str
    user_id: str
    delivery_method: Literal["in_app", "email", "push", "sms"]
    priority: Literal["low", "normal", "high", "urgent"]
    recurrence: Optional[str]  # cron-like or "daily", "weekly", "monthly"
    status: Literal["pending", "validating", "scheduled", "delivering", "delivered", "failed", "snoozed"]
    deliver_at: Optional[str]  # Calculated UTC time
    error: Optional[str]
    delivery_result: Optional[Dict[str, Any]]
    snooze_until: Optional[str]


def create_default_state() -> ReminderState:
    """Create default initial state."""
    return {
        "messages": [],
        "reminder_id": "",
        "content": "",
        "scheduled_time": "",
        "timezone": "UTC",
        "user_id": "",
        "delivery_method": "in_app",
        "priority": "normal",
        "recurrence": None,
        "status": "pending",
        "deliver_at": None,
        "error": None,
        "delivery_result": None,
        "snooze_until": None,
    }


class TimezoneValidationNode:
    """Node for validating and normalizing timezone information."""

    def __init__(self) -> None:
        self.name = "timezone_validation"

    async def __call__(
        self,
        state: ReminderState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Validate timezone and convert scheduled time to UTC."""
        logger.debug("Running timezone_validation node")

        try:
            timezone_str = state.get("timezone", "UTC")
            scheduled_time_str = state.get("scheduled_time", "")

            # Validate timezone
            try:
                tz = pytz.timezone(timezone_str)
            except pytz.UnknownTimeZoneError:
                # Try common aliases
                timezone_mapping = {
                    "ET": "America/New_York",
                    "EST": "America/New_York",
                    "EDT": "America/New_York",
                    "CT": "America/Chicago",
                    "CST": "America/Chicago",
                    "CDT": "America/Chicago",
                    "MT": "America/Denver",
                    "MST": "America/Denver",
                    "MDT": "America/Denver",
                    "PT": "America/Los_Angeles",
                    "PST": "America/Los_Angeles",
                    "PDT": "America/Los_Angeles",
                }
                mapped_tz = timezone_mapping.get(timezone_str.upper())
                if mapped_tz:
                    tz = pytz.timezone(mapped_tz)
                    timezone_str = mapped_tz
                else:
                    logger.warning(f"Unknown timezone: {timezone_str}, defaulting to UTC")
                    tz = pytz.UTC
                    timezone_str = "UTC"

            # Parse scheduled time
            if not scheduled_time_str:
                return {
                    "error": "No scheduled_time provided",
                    "status": "failed",
                }

            try:
                # Try ISO format first
                scheduled_time = datetime.fromisoformat(scheduled_time_str.replace("Z", "+00:00"))
            except ValueError:
                try:
                    # Try dateutil parser
                    scheduled_time = date_parser.parse(scheduled_time_str)
                except Exception as e:
                    return {
                        "error": f"Invalid scheduled_time format: {scheduled_time_str}",
                        "status": "failed",
                    }

            # Localize if naive
            if scheduled_time.tzinfo is None:
                scheduled_time = tz.localize(scheduled_time)

            # Convert to UTC
            deliver_at = scheduled_time.astimezone(pytz.UTC)

            # Check if time is in the past
            now_utc = datetime.now(pytz.UTC)
            if deliver_at < now_utc:
                logger.warning(f"Scheduled time {deliver_at} is in the past")
                return {
                    "error": f"Scheduled time {deliver_at} is in the past",
                    "status": "failed",
                }

            logger.info(f"Reminder scheduled for {deliver_at} UTC (original: {scheduled_time} {timezone_str})")

            return {
                "timezone": timezone_str,
                "deliver_at": deliver_at.isoformat(),
                "status": "scheduled",
                "error": None,
            }

        except Exception as e:
            logger.exception("Timezone validation failed")
            return {
                "error": f"Timezone validation failed: {str(e)}",
                "status": "failed",
            }


class DeliveryTimeCheckNode:
    """Node for checking if it's time to deliver the reminder."""

    def __init__(self) -> None:
        self.name = "delivery_time_check"

    async def __call__(
        self,
        state: ReminderState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Check if it's time to deliver the reminder."""
        logger.debug("Running delivery_time_check node")

        try:
            deliver_at_str = state.get("deliver_at", "")
            snooze_until_str = state.get("snooze_until", "")
            status = state.get("status", "")

            if status == "snoozed" and snooze_until_str:
                # Check if snooze period is over
                try:
                    snooze_until = datetime.fromisoformat(snooze_until_str.replace("Z", "+00:00"))
                    now = datetime.now(pytz.UTC)

                    if snooze_until.tzinfo is None:
                        snooze_until = pytz.UTC.localize(snooze_until)

                    if now < snooze_until:
                        wait_seconds = (snooze_until - now).total_seconds()
                        logger.debug(f"Reminder snoozed, {wait_seconds}s remaining")
                        return {
                            "status": "snoozed",
                            "error": None,
                        }

                    # Snooze is over, proceed to deliver
                    logger.info("Snooze period ended, delivering reminder")
                except Exception as e:
                    logger.error(f"Failed to parse snooze_until: {e}")

            if not deliver_at_str:
                return {
                    "error": "No deliver_at time set",
                    "status": "failed",
                }

            # Parse delivery time
            try:
                deliver_at = datetime.fromisoformat(deliver_at_str.replace("Z", "+00:00"))
            except ValueError:
                return {
                    "error": f"Invalid deliver_at format: {deliver_at_str}",
                    "status": "failed",
                }

            # Ensure UTC
            if deliver_at.tzinfo is None:
                deliver_at = pytz.UTC.localize(deliver_at)

            now = datetime.now(pytz.UTC)

            if now < deliver_at:
                # Not yet time
                wait_seconds = (deliver_at - now).total_seconds()
                logger.debug(f"Reminder not yet due, {wait_seconds}s until delivery")
                return {
                    "status": "scheduled",
                    "error": None,
                }

            # Time to deliver!
            logger.info("Reminder delivery time reached")
            return {
                "status": "delivering",
                "error": None,
            }

        except Exception as e:
            logger.exception("Delivery time check failed")
            return {
                "error": str(e),
                "status": "failed",
            }


class PrepareContentNode:
    """Node for preparing reminder content for delivery."""

    def __init__(self, model: Optional[str] = None) -> None:
        self.name = "prepare_content"
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
                )
            except Exception as e:
                logger.error(f"Failed to initialize OpenAI model: {e}")
                self._llm = None
        return self._llm

    async def __call__(
        self,
        state: ReminderState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Prepare reminder content, potentially enriching it."""
        logger.debug("Running prepare_content node")

        try:
            content = state.get("content", "")
            priority = state.get("priority", "normal")
            delivery_method = state.get("delivery_method", "in_app")

            if not content:
                return {
                    "error": "No reminder content provided",
                    "status": "failed",
                }

            # Format content based on delivery method
            formatted_content = await self._format_content(
                content,
                priority,
                delivery_method,
            )

            return {
                "content": formatted_content,
                "status": "delivering",
            }

        except Exception as e:
            logger.exception("Content preparation failed")
            return {
                "error": str(e),
                "status": "failed",
            }

    async def _format_content(
        self,
        content: str,
        priority: str,
        delivery_method: str,
    ) -> str:
        """Format content for the delivery method."""
        # Add priority indicator for high/urgent
        priority_prefix = ""
        if priority == "urgent":
            priority_prefix = "🚨 URGENT: "
        elif priority == "high":
            priority_prefix = "⚠️ HIGH PRIORITY: "

        # Truncate for SMS
        if delivery_method == "sms":
            max_length = 160 - len(priority_prefix)
            if len(content) > max_length:
                content = content[:max_length - 3] + "..."

        formatted = f"{priority_prefix}{content}"

        # Add footer for email
        if delivery_method == "email":
            formatted += "\n\n---\nThis is an automated reminder from OpenRustClaw."

        return formatted


class DeliverReminderNode:
    """Node for delivering the reminder via the specified channel."""

    def __init__(self) -> None:
        self.name = "deliver_reminder"
        self._delivery_handlers: Dict[str, Any] = {}

    def register_handler(self, method: str, handler: Any) -> None:
        """Register a delivery handler for a method."""
        self._delivery_handlers[method] = handler

    async def __call__(
        self,
        state: ReminderState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Deliver the reminder."""
        logger.debug("Running deliver_reminder node")

        try:
            reminder_id = state.get("reminder_id", "")
            content = state.get("content", "")
            delivery_method = state.get("delivery_method", "in_app")
            user_id = state.get("user_id", "")

            # Get delivery handler
            handler = self._delivery_handlers.get(delivery_method)

            if handler:
                result = await handler(state)
            else:
                # Use default handler
                result = await self._default_delivery(state)

            logger.info(f"Reminder {reminder_id} delivered via {delivery_method}")

            return {
                "delivery_result": result,
                "status": "delivered" if result.get("success") else "failed",
                "error": result.get("error"),
            }

        except Exception as e:
            logger.exception("Reminder delivery failed")
            return {
                "error": str(e),
                "status": "failed",
            }

    async def _default_delivery(self, state: ReminderState) -> Dict[str, Any]:
        """Default delivery handler."""
        delivery_method = state.get("delivery_method", "in_app")
        content = state.get("content", "")
        user_id = state.get("user_id", "")

        # Simulate delivery
        logger.info(f"Delivering reminder to user {user_id} via {delivery_method}: {content[:50]}...")

        return {
            "success": True,
            "method": delivery_method,
            "timestamp": datetime.now(pytz.UTC).isoformat(),
        }


class HandleRecurrenceNode:
    """Node for handling recurring reminders."""

    def __init__(self) -> None:
        self.name = "handle_recurrence"

    async def __call__(
        self,
        state: ReminderState,
        config: Optional[RunnableConfig] = None,
    ) -> Dict[str, Any]:
        """Schedule next occurrence for recurring reminders."""
        logger.debug("Running handle_recurrence node")

        try:
            recurrence = state.get("recurrence")
            deliver_at_str = state.get("deliver_at", "")

            if not recurrence:
                # Not a recurring reminder
                return {}

            # Parse current delivery time
            try:
                deliver_at = datetime.fromisoformat(deliver_at_str.replace("Z", "+00:00"))
                if deliver_at.tzinfo is None:
                    deliver_at = pytz.UTC.localize(deliver_at)
            except Exception:
                logger.error(f"Failed to parse deliver_at: {deliver_at_str}")
                return {}

            # Calculate next occurrence
            next_time = self._calculate_next_occurrence(deliver_at, recurrence)

            if next_time:
                logger.info(f"Scheduling next occurrence at {next_time}")

                # In production, this would create a new reminder job
                return {
                    "next_occurrence": next_time.isoformat(),
                }

            return {}

        except Exception as e:
            logger.exception("Recurrence handling failed")
            return {}

    def _calculate_next_occurrence(
        self,
        current: datetime,
        recurrence: str,
    ) -> Optional[datetime]:
        """Calculate the next occurrence time."""
        recurrence_lower = recurrence.lower()

        if recurrence_lower == "daily":
            return current + timedelta(days=1)
        elif recurrence_lower == "weekly":
            return current + timedelta(weeks=1)
        elif recurrence_lower == "monthly":
            # Approximate monthly
            return current + timedelta(days=30)
        elif recurrence_lower == "hourly":
            return current + timedelta(hours=1)
        elif recurrence_lower.startswith("every "):
            # Parse "every X minutes/hours/days"
            parts = recurrence_lower.split()
            if len(parts) >= 3:
                try:
                    amount = int(parts[1])
                    unit = parts[2]
                    if "minute" in unit:
                        return current + timedelta(minutes=amount)
                    elif "hour" in unit:
                        return current + timedelta(hours=amount)
                    elif "day" in unit:
                        return current + timedelta(days=amount)
                    elif "week" in unit:
                        return current + timedelta(weeks=amount)
                except ValueError:
                    pass

        # Try to parse as cron expression (simplified)
        logger.warning(f"Unsupported recurrence pattern: {recurrence}")
        return None


def should_deliver(state: ReminderState) -> str:
    """Determine if it's time to deliver the reminder."""
    status = state.get("status", "")

    if status == "delivering":
        return "deliver"

    if status == "scheduled":
        return "wait"

    if status == "snoozed":
        return "wait"

    if status in ["failed", "error"]:
        return "error"

    return "deliver"


def should_handle_recurrence(state: ReminderState) -> str:
    """Determine if we should handle recurrence."""
    status = state.get("status", "")
    recurrence = state.get("recurrence")

    if status == "delivered" and recurrence:
        return "recur"

    return "complete"


def build_reminder_graph(
    model: Optional[str] = None,
) -> StateGraph:
    """Build the reminder workflow graph.

    The graph follows this flow:
    1. timezone_validation - Parse and normalize timezone
    2. delivery_time_check - Check if it's time to deliver
    3. prepare_content - Format reminder content
    4. deliver_reminder - Send via appropriate channel
    5. handle_recurrence - Schedule next occurrence if recurring

    Args:
        model: LLM model for content enrichment

    Returns:
        Compiled StateGraph
    """
    # Create nodes
    timezone_validation = TimezoneValidationNode()
    time_check = DeliveryTimeCheckNode()
    prepare_content = PrepareContentNode(model=model)
    deliver = DeliverReminderNode()
    handle_recurrence = HandleRecurrenceNode()

    # Build graph
    workflow = StateGraph(ReminderState)

    # Add nodes
    workflow.add_node("timezone_validation", timezone_validation)
    workflow.add_node("delivery_time_check", time_check)
    workflow.add_node("prepare_content", prepare_content)
    workflow.add_node("deliver_reminder", deliver)
    workflow.add_node("handle_recurrence", handle_recurrence)

    # Add edges
    workflow.add_edge(START, "timezone_validation")

    # From timezone validation to time check if successful
    workflow.add_edge("timezone_validation", "delivery_time_check")

    # Conditional from time check
    workflow.add_conditional_edges(
        "delivery_time_check",
        should_deliver,
        {
            "deliver": "prepare_content",
            "wait": END,  # Not yet time, exit and will be retried
            "error": END,
        },
    )

    workflow.add_edge("prepare_content", "deliver_reminder")

    # Conditional from delivery for recurrence handling
    workflow.add_conditional_edges(
        "deliver_reminder",
        should_handle_recurrence,
        {
            "recur": "handle_recurrence",
            "complete": END,
        },
    )

    workflow.add_edge("handle_recurrence", END)

    return workflow.compile()
