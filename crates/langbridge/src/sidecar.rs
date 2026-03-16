//! Sidecar process lifecycle management.

use std::process::Stdio;
use tokio::process::{Command, Child};
use openrustclaw_core::error::{Error, Result};
use tracing::info;

/// Manages the Python sidecar process lifecycle.
pub struct SidecarManager {
    python_path: String,
    grpc_port: u16,
    child: Option<Child>,
}

impl SidecarManager {
    pub fn new(python_path: String, grpc_port: u16) -> Self {
        Self {
            python_path,
            grpc_port,
            child: None,
        }
    }

    /// Start the sidecar process.
    pub async fn start(&mut self) -> Result<()> {
        let child = Command::new(&self.python_path)
            .arg("-m")
            .arg("openrustclaw_sidecar.server")
            .arg("--port")
            .arg(self.grpc_port.to_string())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::Sidecar(format!("Failed to start sidecar: {}", e)))?;

        info!(port = self.grpc_port, "Python sidecar started");
        self.child = Some(child);
        Ok(())
    }

    /// Stop the sidecar process.
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(ref mut child) = self.child {
            let _ = child.kill().await;
            info!("Python sidecar stopped");
        }
        self.child = None;
        Ok(())
    }

    /// Check if the sidecar is running.
    pub fn is_running(&mut self) -> bool {
        if let Some(ref mut child) = self.child {
            match child.try_wait() {
                Ok(None) => true,  // Still running
                _ => false,
            }
        } else {
            false
        }
    }

    /// Get the gRPC address.
    pub fn grpc_addr(&self) -> String {
        format!("http://127.0.0.1:{}", self.grpc_port)
    }
}
