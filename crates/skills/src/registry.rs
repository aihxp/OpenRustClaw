//! ClawHub Skills Registry
//!
//! A ClawHub-like skill registry for discovering and installing skills.
//! Supports skill search, installation, updates, and signature verification.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use chrono::{DateTime, Duration, Utc};
use ed25519_dalek::{Signature, VerifyingKey, Verifier};
use reqwest::Client;
use semver::{Version, VersionReq};
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqliteConnectOptions, Row, SqlitePool};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

use openrustclaw_core::error::{Error, Result};

/// ClawHub registry client
pub struct ClawHubRegistry {
    client: Client,
    endpoint: String,
    cache: SkillCache,
    local_db: LocalSkillDb,
}

/// Skill metadata from registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillMetadata {
    pub name: String,
    pub version: Version,
    pub description: String,
    pub author: String,
    pub repository: String,
    pub license: String,
    pub keywords: Vec<String>,
    pub categories: Vec<String>,
    pub downloads: u64,
    pub rating: f32,
    pub rating_count: u32,
    pub signature: Option<String>,
    pub published_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub dependencies: Vec<SkillDependency>,
    pub capabilities: Vec<String>,
    pub min_openrustclaw_version: Option<Version>,
}

/// Skill dependency specification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    pub name: String,
    pub version_req: VersionReq,
    pub optional: bool,
}

/// Skill cache for offline access
pub struct SkillCache {
    cache_dir: PathBuf,
    ttl: Duration,
}

/// Local skill database
pub struct LocalSkillDb {
    pool: SqlitePool,
}

/// Search filters
#[derive(Debug, Clone, Default)]
pub struct SearchFilters {
    pub category: Option<String>,
    pub sort_by: SortBy,
    pub min_rating: Option<f32>,
    pub verified_only: bool,
}

/// Sort options for search
#[derive(Debug, Clone, Copy, Default)]
pub enum SortBy {
    #[default]
    Relevance,
    Downloads,
    Rating,
    Recent,
}

impl std::fmt::Display for SortBy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SortBy::Relevance => write!(f, "relevance"),
            SortBy::Downloads => write!(f, "downloads"),
            SortBy::Rating => write!(f, "rating"),
            SortBy::Recent => write!(f, "recent"),
        }
    }
}

/// Install result
#[derive(Debug, Clone)]
pub enum InstallResult {
    AlreadyInstalled,
    Installed {
        name: String,
        version: Version,
        path: PathBuf,
    },
}

/// Update result
#[derive(Debug, Clone)]
pub enum UpdateResult {
    UpToDate,
    Updated { from: Version, to: Version },
}

/// Installed skill info
#[derive(Debug, Clone)]
pub struct InstalledSkill {
    pub name: String,
    pub version: Version,
    pub description: String,
    pub author: String,
    pub install_path: PathBuf,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ClawHubRegistry {
    /// Create a new ClawHub registry client
    pub async fn new(endpoint: impl Into<String>) -> Result<Self> {
        let endpoint = endpoint.into();
        let cache = SkillCache::new().await?;
        let local_db = LocalSkillDb::new().await?;

        Ok(Self {
            client: Client::new(),
            endpoint,
            cache,
            local_db,
        })
    }

    /// Search for skills in the registry
    pub async fn search(&self, query: &str, filters: SearchFilters) -> Result<Vec<SkillMetadata>> {
        let url = format!("{}/api/v1/skills/search", self.endpoint);

        let category = filters.category.unwrap_or_default();
        let sort = filters.sort_by.to_string();

        let response = self
            .client
            .get(&url)
            .query(&[("q", query), ("category", &category), ("sort", &sort)])
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Failed to search registry: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Internal(format!(
                "Registry search failed: {}",
                response.status()
            )));
        }

        let skills: Vec<SkillMetadata> = response
            .json()
            .await
            .map_err(|e| Error::Internal(format!("Failed to parse search results: {}", e)))?;

        // Cache results
        if let Err(e) = self.cache.store_search(query, &skills).await {
            warn!("Failed to cache search results: {}", e);
        }

        Ok(skills)
    }

    /// Get skill details
    pub async fn get_skill(&self, name: &str) -> Result<SkillMetadata> {
        // Check cache first
        if let Some(cached) = self.cache.get(name).await? {
            debug!("Found cached skill metadata for {}", name);
            return Ok(cached);
        }

        let url = format!("{}/api/v1/skills/{}", self.endpoint, name);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Failed to fetch skill: {}", e)))?;

        if response.status().as_u16() == 404 {
            return Err(Error::Internal(format!("Skill '{}' not found", name)));
        }

        if !response.status().is_success() {
            return Err(Error::Internal(format!(
                "Failed to fetch skill: {}",
                response.status()
            )));
        }

        let skill: SkillMetadata = response
            .json()
            .await
            .map_err(|e| Error::Internal(format!("Failed to parse skill metadata: {}", e)))?;

        self.cache.store(name, &skill).await?;
        Ok(skill)
    }

    /// Get skill README
    pub async fn get_readme(&self, name: &str, version: &Version) -> Result<String> {
        let url = format!(
            "{}/api/v1/skills/{}/{}/readme",
            self.endpoint, name, version
        );
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Failed to fetch README: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Internal(format!(
                "Failed to fetch README: {}",
                response.status()
            )));
        }

        response
            .text()
            .await
            .map_err(|e| Error::Internal(format!("Failed to read README: {}", e)))
    }

    /// Download and install a skill
    pub async fn install(&self, name: &str, version: Option<Version>) -> Result<InstallResult> {
        self.install_internal(name, version, &mut std::collections::HashSet::new()).await
    }

    /// Internal install method with dependency tracking
    async fn install_internal(
        &self,
        name: &str,
        version: Option<Version>,
        installing: &mut std::collections::HashSet<String>,
    ) -> Result<InstallResult> {
        // Prevent circular dependencies
        if installing.contains(name) {
            return Err(Error::Internal(format!(
                "Circular dependency detected for skill '{}'",
                name
            )));
        }

        info!("Installing skill: {}", name);
        installing.insert(name.to_string());

        let metadata = self.get_skill(name).await?;

        // Check version compatibility
        if let Some(min_version) = &metadata.min_openrustclaw_version {
            let current = Version::parse(env!("CARGO_PKG_VERSION"))
                .map_err(|e| Error::Internal(format!("Failed to parse current version: {}", e)))?;
            if current < *min_version {
                return Err(Error::Internal(format!(
                    "Skill '{}' requires OpenRustClaw >= {}, but current version is {}",
                    name, min_version, current
                )));
            }
        }

        // Resolve version
        let version = version.unwrap_or_else(|| metadata.version.clone());

        // Check if already installed
        if self.local_db.is_installed(name, &version).await? {
            return Ok(InstallResult::AlreadyInstalled);
        }

        // Download skill package
        let package = self.download_package(name, &version).await?;

        // Verify signature
        if let Some(sig) = &metadata.signature {
            self.verify_signature(name, &package, sig).await?;
        }

        // Resolve and install dependencies (using boxed future to avoid recursion)
        for dep in &metadata.dependencies {
            if !dep.optional {
                info!(
                    "Installing dependency: {} ({})",
                    dep.name, dep.version_req
                );
                let dep_name = dep.name.clone();
                match Box::pin(self.install_internal(&dep_name, None, installing)).await? {
                    InstallResult::AlreadyInstalled => {
                        debug!("Dependency {} already installed", dep.name);
                    }
                    InstallResult::Installed {
                        name,
                        version,
                        path,
                    } => {
                        info!(
                            "Installed dependency {} v{} at {}",
                            name,
                            version,
                            path.display()
                        );
                    }
                }
            }
        }

        // Install skill
        let install_path = self.install_package(name, &version, &package).await?;

        // Register in local DB
        self.local_db
            .register(name, &version, &metadata, &install_path)
            .await?;

        info!(
            "Successfully installed {} v{} at {}",
            name,
            version,
            install_path.display()
        );

        Ok(InstallResult::Installed {
            name: name.to_string(),
            version,
            path: install_path,
        })
    }

    /// Update an installed skill
    pub async fn update(&self, name: &str) -> Result<UpdateResult> {
        info!("Checking for updates: {}", name);

        let installed = self.local_db.get_installed(name).await?;
        let latest = self.get_skill(name).await?;

        if installed.version >= latest.version {
            return Ok(UpdateResult::UpToDate);
        }

        info!(
            "Updating {} from {} to {}",
            name, installed.version, latest.version
        );

        // Install new version
        self.install(name, Some(latest.version.clone())).await?;

        Ok(UpdateResult::Updated {
            from: installed.version,
            to: latest.version,
        })
    }

    /// Uninstall a skill
    pub async fn uninstall(&self, name: &str) -> Result<()> {
        info!("Uninstalling skill: {}", name);

        let installed = self.local_db.get_installed(name).await?;

        // Remove files
        tokio::fs::remove_dir_all(&installed.install_path)
            .await
            .map_err(|e| {
                Error::Internal(format!(
                    "Failed to remove skill directory: {}",
                    e
                ))
            })?;

        // Unregister
        self.local_db.unregister(name).await?;

        info!("Successfully uninstalled {}", name);
        Ok(())
    }

    /// List installed skills
    pub async fn list_installed(&self) -> Result<Vec<InstalledSkill>> {
        self.local_db.list_all().await
    }

    /// Get popular skills
    pub async fn get_popular(&self, limit: usize) -> Result<Vec<SkillMetadata>> {
        let url = format!("{}/api/v1/skills/popular?limit={}", self.endpoint, limit);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Failed to fetch popular skills: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Internal(format!(
                "Failed to fetch popular skills: {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Internal(format!("Failed to parse popular skills: {}", e)))
    }

    /// Get trending skills
    pub async fn get_trending(&self, limit: usize) -> Result<Vec<SkillMetadata>> {
        let url = format!("{}/api/v1/skills/trending?limit={}", self.endpoint, limit);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Failed to fetch trending skills: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Internal(format!(
                "Failed to fetch trending skills: {}",
                response.status()
            )));
        }

        response
            .json()
            .await
            .map_err(|e| Error::Internal(format!("Failed to parse trending skills: {}", e)))
    }

    /// Verify skill signature
    async fn verify_signature(&self, name: &str, package: &[u8], signature: &str) -> Result<()> {
        debug!("Verifying signature for {}", name);

        // Parse signature
        let sig_bytes = BASE64
            .decode(signature)
            .map_err(|e| Error::Security(openrustclaw_core::error::SecurityError::SkillVerificationFailed(format!("Invalid signature format: {}", e))))?;
        
        let signature = Signature::from_slice(&sig_bytes)
            .map_err(|e| Error::Security(openrustclaw_core::error::SecurityError::SkillVerificationFailed(format!("Invalid signature: {}", e))))?;

        // Get author's public key from registry
        let url = format!("{}/api/v1/authors/{}/key", self.endpoint, name);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Failed to fetch author key: {}", e)))?;

        if !response.status().is_success() {
            return Err(Error::Security(
                openrustclaw_core::error::SecurityError::SkillVerificationFailed(
                    "Failed to fetch author public key".to_string(),
                ),
            ));
        }

        let key_b64 = response
            .text()
            .await
            .map_err(|e| Error::Internal(format!("Failed to read author key: {}", e)))?;
        let key_bytes = BASE64
            .decode(key_b64.trim())
            .map_err(|e| Error::Internal(format!("Invalid author key format: {}", e)))?;

        let verifying_key: VerifyingKey = key_bytes
            .as_slice()
            .try_into()
            .map_err(|_| Error::Internal("Invalid public key length".to_string()))?;

        // Verify
        verifying_key
            .verify(package, &signature)
            .map_err(|_| {
                Error::Security(openrustclaw_core::error::SecurityError::SkillVerificationFailed(
                    name.to_string(),
                ))
            })?;

        info!("Signature verified for {}", name);
        Ok(())
    }

    /// Download skill package
    async fn download_package(&self, name: &str, version: &Version) -> Result<Vec<u8>> {
        let url = format!(
            "{}/api/v1/skills/{}/{}/download",
            self.endpoint, name, version
        );
        
        info!("Downloading skill package from {}", url);
        
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Internal(format!("Failed to download package: {}", e)))?;

        if response.status().as_u16() == 404 {
            return Err(Error::Internal(format!(
                "Skill package '{}' v{} not found",
                name, version
            )));
        }

        if !response.status().is_success() {
                return Err(Error::Internal(format!(
                    "Failed to download package: {}",
                    response.status()
                )));
        }

        response
            .bytes()
            .await
            .map(|b| b.to_vec())
            .map_err(|e| Error::Internal(format!("Failed to read package: {}", e)))
    }

    /// Extract and install package
    async fn install_package(
        &self,
        name: &str,
        version: &Version,
        package: &[u8],
    ) -> Result<PathBuf> {
        let base_dir = dirs::data_dir()
            .ok_or_else(|| Error::Internal("Could not determine data directory".to_string()))?
            .join("openrustclaw/skills");

        let install_dir = base_dir.join(format!("{}-{}", name, version));

        // Remove existing installation if present
        if install_dir.exists() {
            tokio::fs::remove_dir_all(&install_dir)
                .await
                .map_err(|e| Error::Internal(format!("Failed to remove old installation: {}", e)))?;
        }

        tokio::fs::create_dir_all(&install_dir)
            .await
            .map_err(|e| Error::Internal(format!("Failed to create install directory: {}", e)))?;

        // Extract tar.gz in a blocking task
        let install_dir_clone = install_dir.clone();
        let package = package.to_vec();
        
        tokio::task::spawn_blocking(move || {
            let tar = flate2::read::GzDecoder::new(&package[..]);
            let mut archive = tar::Archive::new(tar);
            archive
                .unpack(&install_dir_clone)
                .map_err(|e| Error::Internal(format!("Failed to extract package: {}", e)))
        })
        .await
        .map_err(|e| Error::Internal(format!("Extraction task failed: {}", e)))??;

        Ok(install_dir)
    }
}

impl SkillCache {
    /// Create a new skill cache
    async fn new() -> Result<Self> {
        let cache_dir = dirs::cache_dir()
            .ok_or_else(|| Error::Internal("Could not determine cache directory".to_string()))?
            .join("openrustclaw/skills");

        tokio::fs::create_dir_all(&cache_dir)
            .await
            .map_err(|e| Error::Internal(format!("Failed to create cache directory: {}", e)))?;

        Ok(Self {
            cache_dir,
            ttl: Duration::hours(24),
        })
    }

    /// Get cached skill metadata
    async fn get(&self, name: &str) -> Result<Option<SkillMetadata>> {
        let cache_file = self.cache_dir.join(format!("{}.json", name));

        if !cache_file.exists() {
            return Ok(None);
        }

        // Check TTL
        let metadata = tokio::fs::metadata(&cache_file)
            .await
            .map_err(|e| Error::Internal(format!("Failed to read cache metadata: {}", e)))?;

        let modified = metadata
            .modified()
            .map_err(|e| Error::Internal(format!("Failed to get modified time: {}", e)))?;
        let modified: DateTime<Utc> = modified.into();

        if Utc::now() - modified > self.ttl {
            debug!("Cache entry for {} expired", name);
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&cache_file)
            .await
            .map_err(|e| Error::Internal(format!("Failed to read cache: {}", e)))?;

        let skill: SkillMetadata = serde_json::from_str(&content)
            .map_err(|e| Error::Internal(format!("Failed to parse cache: {}", e)))?;

        Ok(Some(skill))
    }

    /// Store skill metadata in cache
    async fn store(&self, name: &str, skill: &SkillMetadata) -> Result<()> {
        let cache_file = self.cache_dir.join(format!("{}.json", name));
        let content = serde_json::to_string_pretty(skill)
            .map_err(|e| Error::Internal(format!("Failed to serialize skill: {}", e)))?;

        tokio::fs::write(&cache_file, content)
            .await
            .map_err(|e| Error::Internal(format!("Failed to write cache: {}", e)))?;

        Ok(())
    }

    /// Store search results
    async fn store_search(&self, query: &str, skills: &[SkillMetadata]) -> Result<()> {
        let cache_file = self
            .cache_dir
            .join(format!("search_{}.json", Self::sanitize_filename(query)));
        let content = serde_json::to_string_pretty(skills)
            .map_err(|e| Error::Internal(format!("Failed to serialize search results: {}", e)))?;

        tokio::fs::write(&cache_file, content)
            .await
            .map_err(|e| Error::Internal(format!("Failed to write search cache: {}", e)))?;

        Ok(())
    }

    /// Sanitize filename for cache
    fn sanitize_filename(query: &str) -> String {
        query
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
            .collect()
    }
}

impl LocalSkillDb {
    /// Create a new local skill database
    async fn new() -> Result<Self> {
        let db_dir = dirs::data_dir()
            .ok_or_else(|| Error::Internal("Could not determine data directory".to_string()))?
            .join("openrustclaw");

        tokio::fs::create_dir_all(&db_dir)
            .await
            .map_err(|e| Error::Internal(format!("Failed to create data directory: {}", e)))?;

        let db_path = db_dir.join("skills.db");
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);

        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| Error::Database(openrustclaw_core::error::DatabaseError::Connection(e.to_string())))?;

        // Initialize schema
        let db = Self { pool };
        db.init_schema().await?;

        Ok(db)
    }

    /// Initialize database schema
    async fn init_schema(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS installed_skills (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL UNIQUE,
                version TEXT NOT NULL,
                description TEXT,
                author TEXT,
                install_path TEXT NOT NULL,
                installed_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_skills_name ON installed_skills(name);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(openrustclaw_core::error::DatabaseError::Migration(e.to_string())))?;

        Ok(())
    }

    /// Check if a skill is installed
    async fn is_installed(&self, name: &str, version: &Version) -> Result<bool> {
        let row: Option<(String,)> = sqlx::query_as(
            "SELECT version FROM installed_skills WHERE name = ?"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(openrustclaw_core::error::DatabaseError::Query(e.to_string())))?;

        if let Some((installed_version,)) = row {
            let installed = Version::parse(&installed_version)
                .map_err(|e| Error::Internal(format!("Invalid version in database: {}", e)))?;
            return Ok(installed == *version);
        }

        Ok(false)
    }

    /// Get installed skill
    async fn get_installed(&self, name: &str) -> Result<InstalledSkill> {
        let row = sqlx::query(
            r#"
            SELECT name, version, description, author, install_path, installed_at, updated_at
            FROM installed_skills
            WHERE name = ?
            "#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Database(openrustclaw_core::error::DatabaseError::Query(e.to_string())))?;

        let row = row.ok_or_else(|| {
            Error::Internal(format!("Skill '{}' is not installed", name))
        })?;

        let version_str: String = row.get("version");
        let install_path_str: String = row.get("install_path");
        let installed_at_str: String = row.get("installed_at");
        let updated_at_str: String = row.get("updated_at");

        Ok(InstalledSkill {
            name: row.get("name"),
            version: Version::parse(&version_str)
                .map_err(|e| Error::Internal(format!("Invalid version: {}", e)))?,
            description: row.get("description"),
            author: row.get("author"),
            install_path: PathBuf::from(install_path_str),
            installed_at: installed_at_str.parse().unwrap_or_else(|_| Utc::now()),
            updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
        })
    }

    /// Register a skill as installed
    async fn register(
        &self,
        name: &str,
        version: &Version,
        metadata: &SkillMetadata,
        install_path: &Path,
    ) -> Result<()> {
        let id = uuid::Uuid::new_v4().to_string();
        let path_str = install_path.to_string_lossy().to_string();

        sqlx::query(
            r#"
            INSERT INTO installed_skills (id, name, version, description, author, install_path)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(name) DO UPDATE SET
                version = excluded.version,
                description = excluded.description,
                author = excluded.author,
                install_path = excluded.install_path,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&id)
        .bind(name)
        .bind(version.to_string())
        .bind(&metadata.description)
        .bind(&metadata.author)
        .bind(&path_str)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Database(openrustclaw_core::error::DatabaseError::Query(e.to_string())))?;

        Ok(())
    }

    /// Unregister a skill
    async fn unregister(&self, name: &str) -> Result<()> {
        sqlx::query("DELETE FROM installed_skills WHERE name = ?")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Database(openrustclaw_core::error::DatabaseError::Query(e.to_string())))?;

        Ok(())
    }

    /// List all installed skills
    async fn list_all(&self) -> Result<Vec<InstalledSkill>> {
        let rows = sqlx::query(
            r#"
            SELECT name, version, description, author, install_path, installed_at, updated_at
            FROM installed_skills
            ORDER BY name
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Database(openrustclaw_core::error::DatabaseError::Query(e.to_string())))?;

        let mut skills = Vec::new();
        for row in rows {
            let version_str: String = row.get("version");
            let install_path_str: String = row.get("install_path");
            let installed_at_str: String = row.get("installed_at");
            let updated_at_str: String = row.get("updated_at");
            
            skills.push(InstalledSkill {
                name: row.get("name"),
                version: Version::parse(&version_str)
                    .map_err(|e| Error::Internal(format!("Invalid version: {}", e)))?,
                description: row.get("description"),
                author: row.get("author"),
                install_path: PathBuf::from(install_path_str),
                installed_at: installed_at_str.parse().unwrap_or_else(|_| Utc::now()),
                updated_at: updated_at_str.parse().unwrap_or_else(|_| Utc::now()),
            });
        }

        Ok(skills)
    }
}

/// Legacy skill registry for backward compatibility
pub struct SkillRegistry {
    skills: HashMap<String, crate::loader::SkillMetadata>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: HashMap::new(),
        }
    }

    /// Register a skill.
    pub fn register(&mut self, skill: crate::loader::SkillMetadata) {
        tracing::info!(name = %skill.name, source = ?skill.source, "Registered skill");
        self.skills.insert(skill.name.clone(), skill);
    }

    /// Get a skill by name.
    pub fn get(&self, name: &str) -> Option<&crate::loader::SkillMetadata> {
        self.skills.get(name)
    }

    /// List all registered skills.
    pub fn list(&self) -> Vec<&crate::loader::SkillMetadata> {
        self.skills.values().collect()
    }

    /// Get skill count.
    pub fn len(&self) -> usize {
        self.skills.len()
    }

    /// Returns `true` if no skills are registered.
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sort_by_display() {
        assert_eq!(SortBy::Relevance.to_string(), "relevance");
        assert_eq!(SortBy::Downloads.to_string(), "downloads");
        assert_eq!(SortBy::Rating.to_string(), "rating");
        assert_eq!(SortBy::Recent.to_string(), "recent");
    }

    #[test]
    fn test_skill_cache_sanitize_filename() {
        assert_eq!(SkillCache::sanitize_filename("hello world"), "hello_world");
        assert_eq!(SkillCache::sanitize_filename("test/skill"), "test_skill");
        assert_eq!(SkillCache::sanitize_filename("skill.name"), "skill_name");
    }
}
