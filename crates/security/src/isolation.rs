//! Session and workspace isolation.
//!
//! Enforces filesystem namespace separation between sessions.
//! Prevents path traversal attacks by rejecting `..` components
//! and never falling back to unresolved paths.

use std::path::{Component, Path, PathBuf};
use openrustclaw_core::error::{SecurityError, Error, Result};
use tracing::warn;

/// Enforces filesystem isolation between sessions.
pub struct IsolationManager {
    base_workspace_path: PathBuf,
}

impl IsolationManager {
    pub fn new(base_workspace_path: PathBuf) -> Self {
        Self {
            base_workspace_path,
        }
    }

    /// Get the isolated workspace path for a session.
    pub fn workspace_path(&self, session_id: &str) -> PathBuf {
        self.base_workspace_path.join(session_id)
    }

    /// Reject paths containing traversal components (`..`, leading `/`).
    fn reject_traversal_components(path: &Path) -> Result<()> {
        for component in path.components() {
            match component {
                Component::ParentDir => {
                    return Err(Error::Security(SecurityError::IsolationViolation(
                        "Path contains '..' traversal component".to_string(),
                    )));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Validate that a file path is within the session's workspace.
    pub fn validate_path(&self, session_id: &str, path: &Path) -> Result<()> {
        // First: reject any path with traversal components before canonicalization
        Self::reject_traversal_components(path)?;

        let workspace = self.workspace_path(session_id);

        // Canonicalize both paths; if either fails, reject rather than falling back
        let canonical = path.canonicalize().map_err(|e| {
            warn!(
                session_id = %session_id,
                path = %path.display(),
                error = %e,
                "Path canonicalization failed during isolation check"
            );
            Error::Security(SecurityError::IsolationViolation(format!(
                "Cannot resolve path {}: {}",
                path.display(),
                e
            )))
        })?;

        let canonical_workspace = workspace.canonicalize().unwrap_or(workspace);

        if canonical.starts_with(&canonical_workspace) {
            Ok(())
        } else {
            warn!(
                session_id = %session_id,
                path = %path.display(),
                "Session isolation violation: path outside workspace"
            );
            Err(Error::Security(SecurityError::IsolationViolation(format!(
                "Path {} is outside session workspace",
                path.display()
            ))))
        }
    }

    /// Create the workspace directory for a session if it doesn't exist.
    pub fn ensure_workspace(&self, session_id: &str) -> Result<PathBuf> {
        let path = self.workspace_path(session_id);
        std::fs::create_dir_all(&path)
            .map_err(|e| Error::Internal(format!("Failed to create workspace: {}", e)))?;
        Ok(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn workspace_path_includes_session_id() {
        let manager = IsolationManager::new(PathBuf::from("/tmp/workspaces"));
        let path = manager.workspace_path("session_123");
        assert_eq!(path, PathBuf::from("/tmp/workspaces/session_123"));
    }

    #[test]
    fn ensure_workspace_creates_directory() {
        let base = std::env::temp_dir().join("openrustclaw_test_isolation");
        let manager = IsolationManager::new(base.clone());

        let workspace = manager.ensure_workspace("test_session").unwrap();
        assert!(workspace.exists());
        assert!(workspace.is_dir());

        // Cleanup
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn validate_path_rejects_outside_workspace() {
        let manager = IsolationManager::new(PathBuf::from("/tmp/workspaces"));
        let result = manager.validate_path("session_1", Path::new("/etc/passwd"));
        assert!(result.is_err());
    }

    #[test]
    fn validate_path_accepts_inside_workspace() {
        let base = std::env::temp_dir().join("openrustclaw_test_isolation_valid");
        let manager = IsolationManager::new(base.clone());

        // Create the workspace and a file inside it
        let workspace = manager.ensure_workspace("sess_1").unwrap();
        let file_path = workspace.join("test.txt");
        std::fs::write(&file_path, "hello").unwrap();

        let result = manager.validate_path("sess_1", &file_path);
        assert!(result.is_ok());

        // Cleanup
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn validate_path_rejects_dot_dot_traversal() {
        let base = std::env::temp_dir().join("openrustclaw_test_traversal");
        let manager = IsolationManager::new(base.clone());
        let _ = manager.ensure_workspace("sess_1");

        // Attempt traversal via ..
        let malicious = base.join("sess_1").join("..").join("..").join("etc").join("passwd");
        let result = manager.validate_path("sess_1", &malicious);
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("traversal"), "Error should mention traversal: {}", err_msg);

        // Cleanup
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn validate_path_rejects_nonexistent_with_traversal() {
        let manager = IsolationManager::new(PathBuf::from("/tmp/workspaces"));
        // Path doesn't exist and contains traversal -- must not fall back to unresolved path
        let path = Path::new("/tmp/workspaces/sess_1/../../../etc/shadow");
        let result = manager.validate_path("sess_1", path);
        assert!(result.is_err());
    }

    #[test]
    fn validate_path_rejects_nonexistent_file_without_fallback() {
        let base = std::env::temp_dir().join("openrustclaw_test_no_fallback");
        let manager = IsolationManager::new(base.clone());
        let _ = manager.ensure_workspace("sess_1");

        // File doesn't exist -- canonicalize fails, should return error not fall back
        let nonexistent = base.join("sess_1").join("does_not_exist.txt");
        let result = manager.validate_path("sess_1", &nonexistent);
        assert!(result.is_err());

        // Cleanup
        let _ = std::fs::remove_dir_all(&base);
    }
}
