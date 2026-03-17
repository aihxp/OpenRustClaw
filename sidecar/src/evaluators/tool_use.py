"""Tool use evaluator."""

from __future__ import annotations

from typing import Any, Iterable, Sequence

from .common import EvaluationResult, clamp_score, extract_tool_names


def evaluate_tool_use(
    required_tools: Sequence[str],
    actual_tool_calls: Iterable[Any],
    disallowed_tools: Sequence[str] | None = None,
) -> EvaluationResult:
    """Evaluate whether the workflow used the expected tools."""
    actual = extract_tool_names(actual_tool_calls)
    required = [tool for tool in required_tools if tool]
    disallowed = set(disallowed_tools or [])

    required_set = set(required)
    actual_set = set(actual)

    missing = sorted(required_set - actual_set)
    unexpected = sorted(actual_set - required_set - disallowed)
    blocked = sorted(actual_set & disallowed)

    if not required_set:
        base_score = 1.0 if not actual else 0.75
    else:
        recall = len(required_set & actual_set) / len(required_set)
        precision = len(required_set & actual_set) / len(actual_set) if actual_set else 0.0
        base_score = (recall * 0.7) + (precision * 0.3)

    penalty = (0.2 * len(blocked)) + (0.05 * len(unexpected))
    score = clamp_score(base_score - penalty)
    passed = not missing and not blocked

    reasoning_parts = []
    if missing:
        reasoning_parts.append(f"missing required tools: {', '.join(missing)}")
    if blocked:
        reasoning_parts.append(f"used disallowed tools: {', '.join(blocked)}")
    if unexpected:
        reasoning_parts.append(f"used extra tools: {', '.join(unexpected)}")
    if not reasoning_parts:
        reasoning_parts.append("tool usage matched expectations")

    return EvaluationResult(
        name="tool_use",
        score=score,
        passed=passed,
        reasoning="; ".join(reasoning_parts),
        details={
            "required_tools": required,
            "actual_tools": actual,
            "missing_tools": missing,
            "unexpected_tools": unexpected,
            "disallowed_tools_used": blocked,
        },
    )
