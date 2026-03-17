//! Live Canvas - A2UI (Agent-to-User Interface) visual workspace
//!
//! This crate provides a real-time visual workspace for agent interaction,
//! enabling dynamic UI elements like text, charts, forms, buttons, and code blocks
//! to be rendered and updated live via WebSocket connections.

pub mod canvas;
pub mod elements;
pub mod error;
pub mod protocol;
pub mod server;

pub use canvas::{Canvas, CanvasAction, CanvasUpdate};
pub use elements::*;
pub use error::{CanvasError, CanvasResult};
pub use protocol::{CanvasCommand, CanvasMessage, CanvasSnapshot, Interaction};
pub use server::CanvasServer;
