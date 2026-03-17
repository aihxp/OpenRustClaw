"""Shared evaluator utilities."""

from __future__ import annotations

import math
import re
from dataclasses import asdict, dataclass, field
from typing import Any, Dict, Iterable, List, Sequence


@dataclass
class EvaluationResult:
    """Normalized evaluation result for workflow quality checks."""

    name: str
    score: float
    passed: bool
    reasoning: str
    details: Dict[str, Any] = field(default_factory=dict)

    def as_dict(self) -> Dict[str, Any]:
        """Convert to a JSON-serializable dictionary."""
        return asdict(self)


def clamp_score(value: float) -> float:
    """Clamp a score to the normalized [0, 1] range."""
    if math.isnan(value) or math.isinf(value):
        return 0.0
    return max(0.0, min(1.0, value))


def normalize_ratio(numerator: float, denominator: float) -> float:
    """Safely compute a ratio."""
    if denominator <= 0:
        return 0.0
    return clamp_score(numerator / denominator)


def tokenize_text(text: str) -> List[str]:
    """Tokenize text into normalized word-like units."""
    return re.findall(r"[a-z0-9_]+", text.lower())


def overlap_score(expected: str, actual: str) -> float:
    """Compute a simple token-overlap recall score."""
    expected_tokens = set(tokenize_text(expected))
    actual_tokens = set(tokenize_text(actual))
    if not expected_tokens:
        return 1.0 if not actual_tokens else 0.0
    return normalize_ratio(len(expected_tokens & actual_tokens), len(expected_tokens))


def average(values: Sequence[float]) -> float:
    """Average a sequence, returning 0 for empty input."""
    if not values:
        return 0.0
    return sum(values) / len(values)


def extract_tool_names(tool_calls: Iterable[Any]) -> List[str]:
    """Extract normalized tool names from strings or tool-call dictionaries."""
    names: List[str] = []
    for call in tool_calls:
        if isinstance(call, str):
            names.append(call)
        elif isinstance(call, dict):
            name = call.get("name")
            if isinstance(name, str) and name:
                names.append(name)
    return names
