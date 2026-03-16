//! Python sidecar gRPC bridge for OpenRustClaw.
//!
//! Connects to the Python LangGraph/LangSmith sidecar via gRPC (tonic).

pub mod client;
pub mod sidecar;

// Generated protobuf code
pub mod proto {
    pub mod orchestration {
        tonic::include_proto!("openrustclaw.orchestration");
    }
    pub mod tracing {
        tonic::include_proto!("openrustclaw.tracing");
    }
}

pub use client::LangBridgeClient;
pub use sidecar::SidecarManager;
