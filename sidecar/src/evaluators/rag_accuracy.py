"""RAG accuracy evaluator."""

from __future__ import annotations

from typing import Any, Iterable, Sequence

from .common import EvaluationResult, average, overlap_score


def evaluate_rag_accuracy(
    answer: str,
    expected_facts: Sequence[str] | None = None,
    cited_source_ids: Iterable[str] | None = None,
    required_source_ids: Sequence[str] | None = None,
) -> EvaluationResult:
    """Evaluate factual coverage and source citation coverage for a RAG answer."""
    facts = [fact for fact in (expected_facts or []) if fact]
    citations = {source_id for source_id in (cited_source_ids or []) if source_id}
    required_sources = {source_id for source_id in (required_source_ids or []) if source_id}

    fact_scores = [overlap_score(fact, answer) for fact in facts]
    fact_coverage = average(fact_scores) if facts else 1.0

    if required_sources:
        citation_coverage = len(required_sources & citations) / len(required_sources)
    else:
        citation_coverage = 1.0

    score = (fact_coverage * 0.75) + (citation_coverage * 0.25)
    missing_sources = sorted(required_sources - citations)
    passed = fact_coverage >= 0.6 and not missing_sources

    reasoning_parts = []
    if facts:
        reasoning_parts.append(f"fact coverage {fact_coverage:.2f}")
    if required_sources:
        reasoning_parts.append(f"citation coverage {citation_coverage:.2f}")
    if missing_sources:
        reasoning_parts.append(f"missing citations: {', '.join(missing_sources)}")
    if not reasoning_parts:
        reasoning_parts.append("no expected facts or source constraints provided")

    return EvaluationResult(
        name="rag_accuracy",
        score=score,
        passed=passed,
        reasoning="; ".join(reasoning_parts),
        details={
            "fact_scores": fact_scores,
            "fact_coverage": fact_coverage,
            "cited_source_ids": sorted(citations),
            "required_source_ids": sorted(required_sources),
            "missing_source_ids": missing_sources,
        },
    )
