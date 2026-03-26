//! Integration tests for the Cursor ACP crate.

use openrustclaw_cursor::tools::{execution_artifact_root, load_execution_artifacts};
use openrustclaw_cursor::{
    ClientConnection, CursorClient, CursorConfig, CursorServer, CursorServerConfig,
    ServerTransport, ToolContext, ToolRegistry, check_cursor_setup, generate_cursor_settings,
    generate_mcp_config, setup_cursor_integration,
};
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_cursor_setup() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_path_buf();

    // Check initial state
    let status = check_cursor_setup(&path).await;
    assert!(!status.all_ready);

    // Setup
    setup_cursor_integration(path.clone()).await.unwrap();

    // Check again
    let status = check_cursor_setup(&path).await;
    assert!(status.all_ready);
    assert!(status.has_cursor_dir);
    assert!(status.has_mcp_config);
    assert!(status.has_settings);
}

#[tokio::test]
async fn test_mcp_config_generation() {
    let config = generate_mcp_config(PathBuf::from("/test/project"), vec!["clippy", "check"]);

    let mcp_servers = config.get("mcpServers").expect("mcpServers key missing");
    let orc = mcp_servers
        .get("openrustclaw")
        .expect("openrustclaw server missing");

    assert_eq!(orc["command"], "cargo");

    let args = orc["args"].as_array().expect("args should be array");
    assert!(args.contains(&serde_json::json!("cursor")));
    assert!(args.contains(&serde_json::json!("serve")));

    assert!(orc["enabled"].as_bool().unwrap());
}

#[tokio::test]
async fn test_cursor_settings_generation() {
    let settings = generate_cursor_settings(PathBuf::from("/test/project"));

    let cursor = settings.get("cursor").expect("cursor key missing");
    assert!(cursor["agent"]["enabled"].as_bool().unwrap());

    let tools = cursor["tools"]["openrustclaw"].clone();
    assert_eq!(tools["projectRoot"], "/test/project");

    let include_patterns = tools["includePatterns"].as_array().unwrap();
    assert!(include_patterns.contains(&serde_json::json!("src/**/*.rs")));
}

#[test]
fn test_cursor_client_creation() {
    let client = CursorClient::with_defaults();
    // Just verify it creates without panic
}

#[test]
fn test_cursor_client_with_tcp() {
    let client = CursorClient::new(
        ClientConnection::Tcp {
            host: "localhost".to_string(),
            port: 8080,
        },
        CursorConfig::default(),
    )
    .with_timeout(60);

    // Verify creation
    assert!(true);
}

#[test]
fn test_cursor_server_creation() {
    let config = CursorServerConfig::default();
    let server = CursorServer::new(config);

    // Just verify it creates without panic
    assert!(true);
}

#[test]
fn test_cursor_server_with_http() {
    let config = CursorServerConfig {
        transport: ServerTransport::Http { port: 8080 },
        name: "test-server".to_string(),
        version: "1.0.0".to_string(),
        cursor_config: CursorConfig::default(),
    };

    let server = CursorServer::new(config);

    // Verify creation
    assert!(true);
}

#[test]
fn test_tool_registry() {
    let ctx = ToolContext::new(PathBuf::from("."), CursorConfig::default());

    let registry = ToolRegistry::new(ctx);
    let tools = registry.list_tools();

    // Should have all default tools
    assert!(!tools.is_empty());

    // Check specific tools exist
    assert!(registry.get("search_code").is_some());
    assert!(registry.get("read_file").is_some());
    assert!(registry.get("edit_file").is_some());
    assert!(registry.get("create_file").is_some());
    assert!(registry.get("delete_file").is_some());
    assert!(registry.get("list_files").is_some());
    assert!(registry.get("run_command").is_some());
    assert!(registry.get("git_status").is_some());
    assert!(registry.get("run_linter").is_some());
}

#[tokio::test]
async fn test_tool_execution_list_files() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_path_buf();

    // Create some test files
    tokio::fs::write(path.join("test1.txt"), "content1")
        .await
        .unwrap();
    tokio::fs::write(path.join("test2.txt"), "content2")
        .await
        .unwrap();
    tokio::fs::create_dir(path.join("subdir")).await.unwrap();

    let ctx = ToolContext::new(path.clone(), CursorConfig::default());
    let registry = ToolRegistry::new(ctx);

    let result = registry
        .execute("list_files", serde_json::json!({"path": "."}))
        .await
        .unwrap();

    let entries = result["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 3); // 2 files + 1 directory
}

#[tokio::test]
async fn test_tool_execution_read_file() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_path_buf();

    tokio::fs::write(path.join("test.txt"), "Hello, World!")
        .await
        .unwrap();
    let ctx = ToolContext::new(path.clone(), CursorConfig::default());
    let registry = ToolRegistry::new(ctx);

    let result = registry
        .execute("read_file", serde_json::json!({"path": "test.txt"}))
        .await
        .unwrap();

    assert_eq!(result["content"], "Hello, World!");
}

#[tokio::test]
async fn test_tool_execution_create_and_delete_file() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_path_buf();
    let ctx = ToolContext::new(path.clone(), CursorConfig::default());
    let registry = ToolRegistry::new(ctx);

    // Create file
    let result = registry
        .execute(
            "create_file",
            serde_json::json!({
                "path": "new_file.txt",
                "content": "New content"
            }),
        )
        .await
        .unwrap();

    assert!(result["created"].as_bool().unwrap());

    // Verify file exists
    let content = tokio::fs::read_to_string(path.join("new_file.txt"))
        .await
        .unwrap();
    assert_eq!(content, "New content");

    // Delete file
    let result = registry
        .execute(
            "delete_file",
            serde_json::json!({
                "path": "new_file.txt",
                "confirm": true
            }),
        )
        .await
        .unwrap();

    assert!(result["deleted"].as_bool().unwrap());

    // Verify file is gone
    assert!(!path.join("new_file.txt").exists());
}

#[tokio::test]
async fn test_tool_execution_edit_file() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_path_buf();

    tokio::fs::write(path.join("test.txt"), "Hello, World!")
        .await
        .unwrap();
    let ctx = ToolContext::new(path.clone(), CursorConfig::default());
    let registry = ToolRegistry::new(ctx);

    let result = registry
        .execute(
            "edit_file",
            serde_json::json!({
                "path": "test.txt",
                "old_text": "World",
                "new_text": "Rust"
            }),
        )
        .await
        .unwrap();

    assert!(result["success"].as_bool().unwrap());

    let content = tokio::fs::read_to_string(path.join("test.txt"))
        .await
        .unwrap();
    assert_eq!(content, "Hello, Rust!");
}

#[tokio::test]
async fn test_tool_execution_search_code() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_path_buf();

    // Create files with search targets
    tokio::fs::write(path.join("file1.rs"), "fn main() {}")
        .await
        .unwrap();
    tokio::fs::write(path.join("file2.rs"), "fn helper() {}")
        .await
        .unwrap();
    let ctx = ToolContext::new(path.clone(), CursorConfig::default());
    let registry = ToolRegistry::new(ctx);

    let result = registry
        .execute(
            "search_code",
            serde_json::json!({
                "query": "fn main",
                "max_results": 10
            }),
        )
        .await
        .unwrap();

    let matches = result["matches"].as_array().unwrap();
    assert!(!matches.is_empty());
    assert_eq!(matches[0]["line_content"], "fn main() {}");
}

#[tokio::test]
async fn test_tool_execution_run_command() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_path_buf();
    let ctx = ToolContext::new(path.clone(), CursorConfig::default());
    let registry = ToolRegistry::new(ctx);

    let result = registry
        .execute(
            "run_command",
            serde_json::json!({
                "command": "echo test-output"
            }),
        )
        .await
        .unwrap();

    assert!(result["success"].as_bool().unwrap());
    assert!(result["stdout"].as_str().unwrap().contains("test-output"));
    assert_eq!(result["exit_code"], 0);
    let artifact_path = result["_artifact"]["path"].as_str().unwrap();
    assert!(std::path::Path::new(artifact_path).exists());

    let artifacts = load_execution_artifacts(&path, 5).unwrap();
    assert_eq!(artifacts[0].tool_name, "run_command");
    assert!(artifacts[0].success);
}

#[tokio::test]
async fn test_edit_file_emits_coding_artifact() {
    let temp_dir = TempDir::new().unwrap();
    let path = temp_dir.path().to_path_buf();

    tokio::process::Command::new("git")
        .args(["init"])
        .current_dir(&path)
        .output()
        .await
        .unwrap();
    tokio::process::Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&path)
        .output()
        .await
        .unwrap();
    tokio::process::Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&path)
        .output()
        .await
        .unwrap();

    tokio::fs::write(path.join("test.txt"), "Hello, World!")
        .await
        .unwrap();
    tokio::process::Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(&path)
        .output()
        .await
        .unwrap();
    tokio::process::Command::new("git")
        .args(["commit", "-m", "initial"])
        .current_dir(&path)
        .output()
        .await
        .unwrap();

    let ctx = ToolContext::new(path.clone(), CursorConfig::default());
    let registry = ToolRegistry::new(ctx);
    let result = registry
        .execute(
            "edit_file",
            serde_json::json!({
                "path": "test.txt",
                "old_text": "World",
                "new_text": "Rust"
            }),
        )
        .await
        .unwrap();

    assert!(result["_artifact"]["path"].as_str().is_some());
    assert!(execution_artifact_root(&path).exists());

    let artifacts = load_execution_artifacts(&path, 5).unwrap();
    let expected_path = path.join("test.txt").to_string_lossy().to_string();
    assert_eq!(artifacts[0].tool_name, "edit_file");
    assert_eq!(
        artifacts[0].target_path.as_deref(),
        Some(expected_path.as_str())
    );
    assert!(
        artifacts[0]
            .diff_preview
            .as_deref()
            .unwrap_or("")
            .contains("+Hello, Rust!")
    );
}

#[tokio::test]
async fn test_config_default() {
    let config = CursorConfig::default();

    assert!(config.enabled);
    assert_eq!(config.terminal_timeout, 30);
    assert_eq!(config.max_file_size, 1024 * 1024);
    assert!(config.auto_format);
    assert!(!config.include_patterns.is_empty());
    assert!(!config.exclude_patterns.is_empty());
}

#[test]
fn test_acp_protocol_version() {
    use openrustclaw_cursor::ACP_PROTOCOL_VERSION;

    // Version should be in semver format
    assert!(!ACP_PROTOCOL_VERSION.is_empty());
    assert!(ACP_PROTOCOL_VERSION.contains('-') || ACP_PROTOCOL_VERSION.contains('.'));
}
