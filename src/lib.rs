//! qianshou - High-performance WebSocket Relay Service Library
//!
//! A local-agent-relay server supporting:
//! - WebSocket relay between clients and agents
//! - JSON-RPC protocol calls
//! - SSE log streaming
//! - Token authentication
//! - Heartbeat keep-alive (30 seconds)

// Re-export commonly used types
pub use serde::{Deserialize, Serialize};
pub use tokio::sync::broadcast;

// JSON-RPC types
pub mod rpc;
pub use rpc::*;

// Authentication types
pub mod auth;
pub use auth::*;

#[cfg(test)]
mod tests {
    #[test]
    fn test_library_imports() {
        // Just verify imports work
        let _ = serde_json::json!({"test": true});
    }
}
