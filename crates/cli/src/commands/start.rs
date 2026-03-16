use anyhow::Result;

pub async fn run(config_path: &str) -> Result<()> {
    println!("Starting OpenRustClaw with config: {}", config_path);
    println!("TODO: Initialize database, start sidecar, start gateway");
    Ok(())
}

pub async fn run_mcp_server(transport: &str) -> Result<()> {
    println!("Starting MCP server (transport: {})", transport);
    println!("TODO: Start MCP server");
    Ok(())
}
