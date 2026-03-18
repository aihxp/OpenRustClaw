//! Rust-native autonomous optimization framework for OpenRustClaw.
//!
//! This crate provides a bounded experiment loop inspired by autoresearch:
//! register a target, submit a candidate change-set, run evals in an isolated
//! temporary workspace, and record promotion decisions with a full audit trail.

pub mod models;
pub mod runner;
pub mod store;

pub use models::{
    CandidateChange, CandidateEvaluationRecord, CandidateRunSummary, CandidateStatus,
    EvaluationCommand, EvaluationSpec, ExecutionTier, ExperimentMetrics, MutationPolicy,
    OptimizationCandidate, OptimizationTarget, PromotionDecision, PromotionEvent, PromotionPolicy,
    RiskClass, ShipStatus, TargetKind, TargetRegistration, TargetUpdate,
};
pub use runner::{CandidateRunner, CandidateRunnerConfig};
pub use store::OptimizationStore;
