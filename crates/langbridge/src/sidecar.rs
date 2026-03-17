//! Sidecar process lifecycle management.

use openrustclaw_core::error::{Error, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpStream;
use tokio::process::{Child, Command};
use tokio::time::{Duration, Instant, sleep};
use tracing::{info, warn};

/// Manages the Python sidecar process lifecycle.
pub struct SidecarManager {
    python_path: String,
    grpc_port: u16,
    env: HashMap<String, String>,
    child: Option<Child>,
}

impl SidecarManager {
    pub fn new(python_path: String, grpc_port: u16) -> Self {
        Self {
            python_path,
            grpc_port,
            env: HashMap::new(),
            child: None,
        }
    }

    /// Add an environment variable for the spawned sidecar process.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Start the sidecar process.
    pub async fn start(&mut self) -> Result<()> {
        let sidecar_dir = sidecar_dir();
        let mut command = Command::new(&self.python_path);
        command
            .arg("-m")
            .arg("src.server")
            .arg("--port")
            .arg(self.grpc_port.to_string())
            .current_dir(&sidecar_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .envs(&self.env);

        let mut child = command
            .spawn()
            .map_err(|e| Error::Sidecar(format!("Failed to start sidecar: {}", e)))?;

        if let Some(stdout) = child.stdout.take() {
            tokio::spawn(pipe_sidecar_output(stdout, false));
        }
        if let Some(stderr) = child.stderr.take() {
            tokio::spawn(pipe_sidecar_output(stderr, true));
        }

        wait_for_sidecar_ready(&mut child, self.grpc_port).await?;

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
                Ok(None) => true, // Still running
                _ => false,
            }
        } else {
            false
        }
    }

    /// Check if the sidecar has a child process (without polling it).
    pub fn is_running_stateless(&self) -> bool {
        self.child.is_some()
    }

    /// Get the gRPC address.
    pub fn grpc_addr(&self) -> String {
        format!("http://127.0.0.1:{}", self.grpc_port)
    }

    /// Get the configured Python path.
    pub fn python_path(&self) -> &str {
        &self.python_path
    }

    /// Get the configured gRPC port.
    pub fn grpc_port(&self) -> u16 {
        self.grpc_port
    }
}

fn sidecar_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sidecar")
}

async fn wait_for_sidecar_ready(child: &mut Child, grpc_port: u16) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(Error::Sidecar(format!(
                    "Sidecar exited before becoming ready (status: {})",
                    status
                )));
            }
            Ok(None) => {}
            Err(e) => {
                return Err(Error::Sidecar(format!(
                    "Failed to poll sidecar process state: {}",
                    e
                )));
            }
        }

        if TcpStream::connect(("127.0.0.1", grpc_port)).await.is_ok() {
            return Ok(());
        }

        if Instant::now() >= deadline {
            let _ = child.kill().await;
            return Err(Error::Sidecar(format!(
                "Sidecar did not become ready on 127.0.0.1:{} within 5s",
                grpc_port
            )));
        }

        sleep(Duration::from_millis(100)).await;
    }
}

async fn pipe_sidecar_output<T>(stream: T, is_stderr: bool)
where
    T: tokio::io::AsyncRead + Unpin,
{
    let mut lines = BufReader::new(stream).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        if is_stderr {
            warn!(target: "openrustclaw_sidecar", "{}", line);
        } else {
            info!(target: "openrustclaw_sidecar", "{}", line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sidecar_manager_new() {
        let manager = SidecarManager::new("python3".to_string(), 50051);
        assert_eq!(manager.python_path(), "python3");
        assert_eq!(manager.grpc_port(), 50051);
        assert!(!manager.is_running_stateless());
    }

    #[test]
    fn test_sidecar_manager_grpc_addr_default_port() {
        let manager = SidecarManager::new("python3".to_string(), 50051);
        assert_eq!(manager.grpc_addr(), "http://127.0.0.1:50051");
    }

    #[test]
    fn test_sidecar_manager_grpc_addr_custom_port() {
        let manager = SidecarManager::new("python3".to_string(), 9999);
        assert_eq!(manager.grpc_addr(), "http://127.0.0.1:9999");
    }

    #[test]
    fn test_sidecar_manager_grpc_addr_zero_port() {
        let manager = SidecarManager::new("python3".to_string(), 0);
        assert_eq!(manager.grpc_addr(), "http://127.0.0.1:0");
    }

    #[test]
    fn test_sidecar_manager_grpc_addr_max_port() {
        let manager = SidecarManager::new("python3".to_string(), u16::MAX);
        assert_eq!(manager.grpc_addr(), "http://127.0.0.1:65535");
    }

    #[test]
    fn test_sidecar_manager_custom_python_path() {
        let manager = SidecarManager::new("/usr/bin/python3.11".to_string(), 50051);
        assert_eq!(manager.python_path(), "/usr/bin/python3.11");
    }

    #[test]
    fn test_sidecar_manager_not_running_initially() {
        let manager = SidecarManager::new("python3".to_string(), 50051);
        assert!(!manager.is_running_stateless());
    }

    #[test]
    fn test_sidecar_manager_grpc_addr_format() {
        let manager = SidecarManager::new("python3".to_string(), 8080);
        let addr = manager.grpc_addr();
        assert!(addr.starts_with("http://"));
        assert!(addr.contains("127.0.0.1"));
        assert!(addr.ends_with(":8080"));
    }

    #[test]
    fn test_sidecar_manager_different_ports_different_addrs() {
        let m1 = SidecarManager::new("python3".to_string(), 50051);
        let m2 = SidecarManager::new("python3".to_string(), 50052);
        assert_ne!(m1.grpc_addr(), m2.grpc_addr());
    }

    #[test]
    fn test_sidecar_manager_with_env() {
        let manager =
            SidecarManager::new("python3".to_string(), 50051).with_env("FOO", "bar");
        assert_eq!(manager.env.get("FOO").map(String::as_str), Some("bar"));
    }
}
