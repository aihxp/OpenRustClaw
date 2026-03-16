//! Messaging platform integrations for OpenRustClaw.
//!
//! v1 supports WebChat only. Telegram, Discord, Slack deferred to v2.

pub mod webchat;

pub use webchat::WebChatChannel;
