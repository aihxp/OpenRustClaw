"""Memory recall evaluator."""

from __future__ import annotations

from typing import Sequence

from .common import EvaluationResult, average, overlap_score


def evaluate_memory_recall(
    recalled_text: str,
    expected_memories: Sequence[str],
    forbidden_memories: Sequence[str] | None = None,
) -> EvaluationResult:
    """Evaluate whether recalled content includes expected memories and excludes forbidden ones."""
    expected = [memory for memory in expected_memories if memory]
    forbidden = [memory for memory in (forbidden_memories or []) if memory]

    expected_scores = [overlap_score(memory, recalled_text) for memory in expected]
    recall_score = average(expected_scores) if expected else 1.0

    forbidden_scores = [overlap_score(memory, recalled_text) for memory in forbidden]
    leakage_score = average(forbidden_scores) if forbidden else 0.0

    score = max(0.0, (recall_score * 0.8) + ((1.0 - leakage_score) * 0.2))
    leaking = [memory for memory, hit in zip(forbidden, forbidden_scores) if hit >= 0.5]
    passed = recall_score >= 0.6 and not leaking

    reasoning_parts = [f"memory recall {recall_score:.2f}"]
    if forbidden:
        reasoning_parts.append(f"forbidden leakage {leakage_score:.2f}")
    if leaking:
        reasoning_parts.append(f"leaked forbidden memories: {len(leaking)}")

    return EvaluationResult(
        name="memory_recall",
        score=score,
        passed=passed,
        reasoning="; ".join(reasoning_parts),
        details={
            "expected_memories": expected,
            "expected_scores": expected_scores,
            "forbidden_memories": forbidden,
            "forbidden_scores": forbidden_scores,
            "leaked_forbidden_memories": leaking,
        },
    )
