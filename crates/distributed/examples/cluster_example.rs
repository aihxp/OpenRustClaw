//! Example of using OpenRustClaw distributed mode.
use openrustclaw_distributed::{ClusterManagerBuilder, NodeRole};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Example: Start as leader
    let manager = ClusterManagerBuilder::new()
        .node_id("leader-1")
        .bootstrap_leader()
        .listen_addr("0.0.0.0:50051".parse()?)
        .api_addr("0.0.0.0:8080".parse()?)
        .with_gossip()
        .build()
        .await?;

    println!("Leader node ID: {}", manager.node_id());

    // Example: Start as worker
    let leader_addr: SocketAddr = "127.0.0.1:50051".parse()?;
    let worker = ClusterManagerBuilder::new()
        .node_id("worker-1")
        .join_worker(leader_addr)
        .listen_addr("0.0.0.0:50052".parse()?)
        .build()
        .await?;

    println!("Worker node ID: {}", worker.node_id());

    Ok(())
}
