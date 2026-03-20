//! Skill/plugin system for OpenRustClaw.
//!
//! Supports SKILL.md format (progressive disclosure), Ed25519 verification,
//! and WASM sandbox scaffolding for untrusted skills.

pub mod capabilities;
pub mod compiler;
pub mod extension;
pub mod loader;
pub mod marketplace;
pub mod registry;
pub mod runtime;
pub mod sandbox;

pub use capabilities::{
    canonical_capability_name, declared_sensitive_capability_names, is_sensitive_capability,
    normalize_capability_names, parse_capability_name, parse_capability_names,
    sensitive_capabilities,
};
pub use compiler::{
    CompiledCliSchema, CompiledHelpIndex, CompiledMcpSchema, CompiledSkillArtifact,
    CompiledSkillManifest, CompiledSkillScanFinding, CompiledSkillScanReport, CompiledSkillStatus,
    FindingSeverity, compile_skill_file, compile_skill_to_dir, list_compiled_manifests,
    load_compiled_artifact, remove_compiled_artifact,
};
pub use extension::{
    ExtensionComponentBinding, ExtensionImplementationStatus, ExtensionManifest,
    ExtensionToolBinding, ExtensionTriggerBinding, build_extension_manifest,
    list_extension_manifests, load_extension_manifest,
};
pub use loader::SkillLoader;
pub use marketplace::{MarketplaceClient, MarketplaceListing};
pub use registry::{
    ClawHubRegistry, InstallResult, InstalledSkill, SearchFilters, SkillCache, SkillDependency,
    SkillMetadata, SkillRegistry, SortBy, UpdateResult,
};
pub use runtime::{
    CompiledBackgroundService, CompiledSkillExecutionResult, compiled_skill_background_services,
    compiled_skill_executable_components, execute_compiled_skill_artifact,
    resolve_compiled_skill_background_service,
};
