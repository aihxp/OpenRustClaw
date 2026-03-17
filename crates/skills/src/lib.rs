//! Skill/plugin system for OpenRustClaw.
//!
//! Supports SKILL.md format (progressive disclosure), Ed25519 verification,
//! and WASM sandboxing for untrusted skills.

pub mod loader;
pub mod marketplace;
pub mod registry;
pub mod sandbox;

pub use loader::SkillLoader;
pub use marketplace::{MarketplaceClient, MarketplaceListing};
pub use registry::{
    ClawHubRegistry, InstallResult, InstalledSkill, SearchFilters, SkillCache, SkillDependency,
    SkillMetadata, SkillRegistry, SortBy, UpdateResult,
};
