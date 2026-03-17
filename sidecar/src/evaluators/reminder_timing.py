"""Reminder timing evaluator."""

from __future__ import annotations

from datetime import datetime
from typing import Optional

from .common import EvaluationResult, clamp_score


def _parse_timestamp(value: str) -> datetime:
    return datetime.fromisoformat(value.replace("Z", "+00:00"))


def evaluate_reminder_timing(
    scheduled_time: str,
    delivered_time: str,
    allowed_drift_seconds: float = 60.0,
) -> EvaluationResult:
    """Evaluate whether a reminder fired close to the intended delivery time."""
    try:
        scheduled = _parse_timestamp(scheduled_time)
        delivered = _parse_timestamp(delivered_time)
    except ValueError as exc:
        return EvaluationResult(
            name="reminder_timing",
            score=0.0,
            passed=False,
            reasoning=f"invalid timestamp: {exc}",
            details={
                "scheduled_time": scheduled_time,
                "delivered_time": delivered_time,
            },
        )

    drift_seconds = abs((delivered - scheduled).total_seconds())
    if allowed_drift_seconds <= 0:
        score = 1.0 if drift_seconds == 0 else 0.0
    else:
        score = clamp_score(1.0 - (drift_seconds / allowed_drift_seconds))

    passed = drift_seconds <= allowed_drift_seconds
    reasoning = (
        f"delivery drift {drift_seconds:.2f}s"
        if passed
        else f"delivery drift {drift_seconds:.2f}s exceeds {allowed_drift_seconds:.2f}s"
    )

    return EvaluationResult(
        name="reminder_timing",
        score=score,
        passed=passed,
        reasoning=reasoning,
        details={
            "scheduled_time": scheduled.isoformat(),
            "delivered_time": delivered.isoformat(),
            "drift_seconds": drift_seconds,
            "allowed_drift_seconds": allowed_drift_seconds,
        },
    )
