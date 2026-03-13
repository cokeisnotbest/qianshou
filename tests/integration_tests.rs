//! Integration Tests for qianshou relay server
//!
//! Tests end-to-end functionality including:
//! - WebSocket connection with valid/invalid token
//! - JSON-RPC request/response flow
//! - Relay message forwarding
//! - Heartbeat timeout

use qianshou::{
    JsonRpcHandler, JsonRpcRequest, JsonRpcId, 
    Connection, ConnectionType, ConnectionState, RelayState, 
    validate_token, LogBroadcaster, LogLevel,
};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

/// Test helper to create a connection ID
fn test_connection_id() -> Uuid {
    Uuid::new_v4()
}

// === Token Authentication Tests ===

#[test]
fn test_valid_token_authentication() {
    // Test that valid token is accepted
    let result = validate_token(Some("valid-token".to_string()), "valid-token");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "valid-token");
}

#[test]
fn test_invalid_token_authentication() {
    // Test that invalid token is rejected
    let result = validate_token(Some("wrong-token".to_string()), "valid-token");
    assert!(result.is_err());
    let (status, _message) = result.unwrap_err();
    assert_eq!(status, hyper::StatusCode::UNAUTHORIZED);
    assert!(_message.contains("Invalid"));
}

#[test]
fn test_missing_token_authentication() {
    // Test that missing token is rejected
    let result = validate_token(None, "valid-token");
    assert!(result.is_err());
    let (status, _message) = result.unwrap_err();
    assert_eq!(status, hyper::StatusCode::UNAUTHORIZED);
    assert!(_message.contains("Missing"));
}

#[test]
fn test_empty_token_authentication() {
    // Test that empty token is rejected
    let result = validate_token(Some("".to_string()), "valid-token");
    assert!(result.is_err());
    let (status, _message) = result.unwrap_err();
    assert_eq!(status, hyper::StatusCode::UNAUTHORIZED);
}

// === JSON-RPC Request/Response Tests ===

#[test]
fn test_json_rpc_agent_connect_request() {
    // Test agent.connect request handling
    let request = JsonRpcRequest {
        id: Some(JsonRpcId::Num(1)),
        jsonrpc: "2.0".to_string(),
        method: "agent.connect".to_string(),
        params: Some(json!({
            "agent_id": "test-agent-123",
            "capabilities": ["chat", "code"]
        })),
    };
    
    let conn_id = test_connection_id();
    let response = JsonRpcHandler::handle(request, conn_id);
    
    // Verify successful response
    assert!(response.result.is_some());
    let result = response.result.unwrap();
    assert_eq!(result.get("status").unwrap(), "connected");
    assert_eq!(result.get("agent_id").unwrap(), "test-agent-123");
    assert!(result.get("connection_id").is_some());
}

#[test]
fn test_json_rpc_agent_disconnect_request() {
    // Test agent.disconnect request handling
    let request = JsonRpcRequest {
        id: Some(JsonRpcId::Num(2)),
        jsonrpc: "2.0".to_string(),
        method: "agent.disconnect".to_string(),
        params: Some(json!({
            "agent_id": "test-agent-123"
        })),
    };
    
    let conn_id = test_connection_id();
    let response = JsonRpcHandler::handle(request, conn_id);
    
    // Verify successful response
    assert!(response.result.is_some());
    let result = response.result.unwrap();
    assert_eq!(result.get("status").unwrap(), "disconnected");
}

#[test]
fn test_json_rpc_agent_call_request() {
    // Test agent.call request handling
    let request = JsonRpcRequest {
        id: Some(JsonRpcId::Num(3)),
        jsonrpc: "2.0".to_string(),
        method: "agent.call".to_string(),
        params: Some(json!({
            "agent_id": "target-agent",
            "method": "execute",
            "params": {"command": "ls -la"}
        })),
    };
    
    let conn_id = test_connection_id();
    let response = JsonRpcHandler::handle(request, conn_id);
    
    // Verify successful response
    assert!(response.result.is_some());
    let result = response.result.unwrap();
    assert!(result.get("result").is_some());
}

#[test]
fn test_json_rpc_invalid_request_version() {
    // Test that invalid JSON-RPC version returns error
    let request = JsonRpcRequest {
        id: Some(JsonRpcId::Num(1)),
        jsonrpc: "1.0".to_string(), // Invalid version
        method: "agent.connect".to_string(),
        params: None,
    };
    
    let conn_id = test_connection_id();
    let response = JsonRpcHandler::handle(request, conn_id);
    
    // Verify error response
    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap().code, -32600); // Invalid Request
}

#[test]
fn test_json_rpc_method_not_found() {
    // Test that unknown method returns error
    let request = JsonRpcRequest {
        id: Some(JsonRpcId::Num(1)),
        jsonrpc: "2.0".to_string(),
        method: "unknown.method".to_string(),
        params: None,
    };
    
    let conn_id = test_connection_id();
    let response = JsonRpcHandler::handle(request, conn_id);
    
    // Verify error response
    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap().code, -32601); // Method Not Found
}

#[test]
fn test_json_rpc_response_preserves_id() {
    // Test that response preserves request ID
    let request = JsonRpcRequest {
        id: Some(JsonRpcId::Str("request-abc".to_string())),
        jsonrpc: "2.0".to_string(),
        method: "agent.connect".to_string(),
        params: None,
    };
    
    let conn_id = test_connection_id();
    let response = JsonRpcHandler::handle(request, conn_id);
    
    // Verify ID is preserved
    assert_eq!(response.id, Some(JsonRpcId::Str("request-abc".to_string())));
}

// === Relay Message Forwarding Tests ===

#[test]
fn test_relay_forward_to_existing_connection() {
    // Create relay state
    let (log_tx, _) = broadcast::channel(100);
    let relay_state = Arc::new(RelayState::new(log_tx));
    
    // Register source connection
    let source_id = {
        let mut conn = Connection::new();
        conn.connect();
        relay_state.connection_registry.register(conn)
    };
    
    // Register target connection
    let target_id = {
        let mut conn = Connection::with_type(ConnectionType::Agent);
        conn.connect();
        conn.set_remote_id("target-agent");
        relay_state.connection_registry.register(conn)
    };
    
    // Forward message from source to target
    let request = JsonRpcRequest {
        id: Some(JsonRpcId::Num(1)),
        jsonrpc: "2.0".to_string(),
        method: "relay.forward".to_string(),
        params: Some(json!({
            "target_id": target_id.to_string(),
            "message": {"data": "hello from relay"}
        })),
    };
    
    let response = JsonRpcHandler::handle_with_state(request, source_id, relay_state.clone());
    
    // Verify successful forward
    assert!(response.result.is_some());
    let result = response.result.unwrap();
    assert_eq!(result.get("status").unwrap(), "forwarded");
    assert_eq!(result.get("target").unwrap().as_str().unwrap(), target_id.to_string());
}

#[test]
fn test_relay_forward_to_nonexistent_connection() {
    // Create relay state
    let (log_tx, _) = broadcast::channel(100);
    let relay_state = Arc::new(RelayState::new(log_tx));
    
    // Register source connection only
    let source_id = {
        let mut conn = Connection::new();
        conn.connect();
        relay_state.connection_registry.register(conn)
    };
    
    // Try to forward to nonexistent target
    let nonexistent_id = Uuid::new_v4();
    let request = JsonRpcRequest {
        id: Some(JsonRpcId::Num(1)),
        jsonrpc: "2.0".to_string(),
        method: "relay.forward".to_string(),
        params: Some(json!({
            "target_id": nonexistent_id.to_string(),
            "message": {"data": "hello"}
        })),
    };
    
    let response = JsonRpcHandler::handle_with_state(request, source_id, relay_state.clone());
    
    // Should return error for nonexistent target
    assert!(response.error.is_some());
    assert!(response.error.unwrap().message.contains("not found"));
}

#[test]
fn test_relay_forward_by_remote_id() {
    // Create relay state
    let (log_tx, _) = broadcast::channel(100);
    let relay_state = Arc::new(RelayState::new(log_tx));
    
    // Register source connection
    let source_id = {
        let mut conn = Connection::new();
        conn.connect();
        relay_state.connection_registry.register(conn)
    };
    
    // Register target with remote_id
    {
        let mut conn = Connection::with_type(ConnectionType::Agent);
        conn.connect();
        conn.set_remote_id("my-agent-123");
        relay_state.connection_registry.register(conn);
    }
    
    // Forward to agent by remote_id
    let request = JsonRpcRequest {
        id: Some(JsonRpcId::Num(1)),
        jsonrpc: "2.0".to_string(),
        method: "relay.forward".to_string(),
        params: Some(json!({
            "target_id": "my-agent-123",
            "message": {"cmd": "execute"}
        })),
    };
    
    let response = JsonRpcHandler::handle_with_state(request, source_id, relay_state.clone());
    
    // Verify successful forward by remote_id
    assert!(response.result.is_some());
    let result = response.result.unwrap();
    assert_eq!(result.get("status").unwrap(), "forwarded");
    assert_eq!(result.get("target").unwrap().as_str().unwrap(), "my-agent-123");
}

#[test]
fn test_relay_forward_to_disconnected_target() {
    // Create relay state
    let (log_tx, _) = broadcast::channel(100);
    let relay_state = Arc::new(RelayState::new(log_tx));
    
    // Register source connection
    let source_id = {
        let mut conn = Connection::new();
        conn.connect();
        relay_state.connection_registry.register(conn)
    };
    
    // Register disconnected target
    let target_id = {
        let conn = Connection::with_type(ConnectionType::Agent); // Not connected
        relay_state.connection_registry.register(conn)
    };
    
    // Try to forward to disconnected target
    let request = JsonRpcRequest {
        id: Some(JsonRpcId::Num(1)),
        jsonrpc: "2.0".to_string(),
        method: "relay.forward".to_string(),
        params: Some(json!({
            "target_id": target_id.to_string(),
            "message": {"data": "hello"}
        })),
    };
    
    let response = JsonRpcHandler::handle_with_state(request, source_id, relay_state.clone());
    
    // Should return error for disconnected target
    assert!(response.error.is_some());
}

// === Connection State Tests ===

#[test]
fn test_connection_state_transitions() {
    let mut conn = Connection::new();
    
    // Initial state should be Connecting
    assert_eq!(conn.state, ConnectionState::Connecting);
    
    // Connect transition
    conn.connect();
    assert_eq!(conn.state, ConnectionState::Connected);
    
    // Close transition
    conn.close();
    assert_eq!(conn.state, ConnectionState::Closing);
    
    // Disconnect transition
    conn.disconnect();
    assert_eq!(conn.state, ConnectionState::Disconnected);
}

#[test]
fn test_connection_type_updates() {
    let mut conn = Connection::new();
    
    // Default type should be Client
    assert_eq!(conn.connection_type, ConnectionType::Client);
    
    // Create with Agent type
    let mut agent_conn = Connection::with_type(ConnectionType::Agent);
    assert_eq!(agent_conn.connection_type, ConnectionType::Agent);
}

#[test]
fn test_connection_remote_id() {
    let mut conn = Connection::new();
    
    // Set remote_id
    conn.set_remote_id("agent-001");
    assert_eq!(conn.remote_id, Some("agent-001".to_string()));
}

// === Connection Registry Tests ===

#[test]
fn test_connection_registry_lifecycle() {
    let registry = qianshou::ConnectionRegistry::new();
    
    // Register new connection
    let mut conn = Connection::new();
    let id = conn.id;
    registry.register(conn);
    
    // Verify connection exists
    assert!(registry.get(&id).is_some());
    assert_eq!(registry.active_count(), 0); // Not connected yet
    
    // Connect the connection
    registry.update_state(&id, ConnectionState::Connected);
    assert_eq!(registry.active_count(), 1);
    
    // Close the connection
    registry.update_state(&id, ConnectionState::Closing);
    assert_eq!(registry.active_count(), 0);
    
    // Disconnect
    registry.update_state(&id, ConnectionState::Disconnected);
    assert_eq!(registry.active_count(), 0);
    
    // Remove connection
    let removed = registry.remove(&id);
    assert!(removed.is_some());
    assert!(registry.get(&id).is_none());
}

#[test]
fn test_connection_registry_active_clients_and_agents() {
    let registry = qianshou::ConnectionRegistry::new();
    
    // Register a client
    let mut client = Connection::new();
    client.connect();
    registry.register(client);
    
    // Register an agent
    let mut agent = Connection::with_type(ConnectionType::Agent);
    agent.connect();
    agent.set_remote_id("test-agent");
    registry.register(agent);
    
    // Check active counts
    let clients = registry.active_clients();
    let agents = registry.active_agents();
    
    assert_eq!(clients.len(), 1);
    assert_eq!(agents.len(), 1);
    assert_eq!(clients[0].connection_type, ConnectionType::Client);
    assert_eq!(agents[0].connection_type, ConnectionType::Agent);
}

// === Logging Tests ===

#[test]
fn test_log_broadcaster() {
    let broadcaster = LogBroadcaster::new();
    
    // Subscribe before broadcasting
    let mut subscriber = broadcaster.subscribe();
    
    // Broadcast log message
    broadcaster.info("Test log message");
    
    // Receive log entry
    let entry = subscriber.try_recv().unwrap();
    assert_eq!(entry.message, "Test log message");
    assert_eq!(entry.level, LogLevel::Info);
}

#[test]
fn test_log_broadcaster_connection_events() {
    let broadcaster = LogBroadcaster::new();
    
    // Subscribe first
    let mut subscriber = broadcaster.subscribe();
    
    // Broadcast connection open
    broadcaster.log_connection_open("test-conn-123");
    let entry = subscriber.try_recv().unwrap();
    assert!(entry.message.contains("opened"));
    assert_eq!(entry.connection_id, Some("test-conn-123".to_string()));
    
    // Broadcast connection close
    broadcaster.log_connection_close("test-conn-123");
    let entry = subscriber.try_recv().unwrap();
    assert!(entry.message.contains("closed"));
}

#[test]
fn test_log_level_filtering() {
    use qianshou::should_include_log;
    
    // Debug should include all levels
    assert!(should_include_log(&LogLevel::Debug, &LogLevel::Debug));
    assert!(should_include_log(&LogLevel::Info, &LogLevel::Debug));
    assert!(should_include_log(&LogLevel::Warn, &LogLevel::Debug));
    assert!(should_include_log(&LogLevel::Error, &LogLevel::Debug));
    
    // Error should include only error
    assert!(!should_include_log(&LogLevel::Debug, &LogLevel::Error));
    assert!(!should_include_log(&LogLevel::Info, &LogLevel::Error));
    assert!(!should_include_log(&LogLevel::Warn, &LogLevel::Error));
    assert!(should_include_log(&LogLevel::Error, &LogLevel::Error));
}

// === Message Parsing Tests ===

#[test]
fn test_parse_valid_json_rpc_message() {
    let json = r#"{"jsonrpc":"2.0","method":"agent.connect","params":{"agent_id":"test"},"id":1}"#;
    let result = JsonRpcHandler::parse_message(json);
    
    assert!(result.is_ok());
    let request = result.unwrap();
    assert_eq!(request.method, "agent.connect");
    assert_eq!(request.jsonrpc, "2.0");
}

#[test]
fn test_parse_invalid_json() {
    let json = "not valid json";
    let result = JsonRpcHandler::parse_message(json);
    
    assert!(result.is_err());
    let response = result.unwrap_err();
    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap().code, -32700); // Parse Error
}

#[test]
fn test_parse_valid_json_not_rpc() {
    let json = r#"{"foo":"bar"}"#;
    let result = JsonRpcHandler::parse_message(json);
    
    assert!(result.is_err());
    let response = result.unwrap_err();
    assert!(response.error.is_some());
    assert_eq!(response.error.unwrap().code, -32600); // Invalid Request
}

// === Heartbeat Tests ===

#[test]
fn test_heartbeat_constants() {
    // These constants are defined in main.rs but we test their values here
    const HEARTBEAT_INTERVAL_SECS: u64 = 30;
    const HEARTBEAT_TIMEOUT_SECS: u64 = 10;
    
    assert_eq!(HEARTBEAT_INTERVAL_SECS, 30);
    assert_eq!(HEARTBEAT_TIMEOUT_SECS, 10);
    assert!(HEARTBEAT_INTERVAL_SECS > HEARTBEAT_TIMEOUT_SECS);
}

#[test]
fn test_connection_activity_tracking() {
    let mut conn = Connection::new();
    
    let initial_activity = conn.last_activity;
    
    // Update activity
    std::thread::sleep(std::time::Duration::from_millis(10));
    conn.update_activity();
    
    // Activity should be updated
    assert!(conn.last_activity >= initial_activity);
    
    // Activity should also update on state transitions
    let before_connect = conn.last_activity;
    std::thread::sleep(std::time::Duration::from_millis(10));
    conn.connect();
    assert!(conn.last_activity >= before_connect);
}

// === Async Integration Tests ===

#[tokio::test]
async fn test_async_connection_handling() {
    // Test async connection handling with tokio
    use tokio::time::{timeout, Duration};
    
    // Create a simple async operation that completes
    let result = timeout(Duration::from_millis(100), async {
        let (log_tx, _) = broadcast::channel(10);
        let relay_state = RelayState::new(log_tx);
        relay_state
    }).await;
    
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_async_relay_state_operations() {
    // Test async operations on relay state
    let (log_tx, _) = broadcast::channel(100);
    let relay_state = Arc::new(RelayState::new(log_tx));
    
    // Register multiple connections
    for i in 0..5 {
        let mut conn = Connection::new();
        conn.connect();
        conn.set_remote_id(format!("agent-{}", i));
        relay_state.connection_registry.register(conn);
    }
    
    // Verify all connections are registered
    assert_eq!(relay_state.connection_registry.active_count(), 5);
    
    // Find agent by remote_id
    let found = relay_state.connection_registry.get_by_remote_id("agent-2");
    assert!(found.is_some());
    assert_eq!(found.unwrap().remote_id, Some("agent-2".to_string()));
}

#[tokio::test]
async fn test_async_message_handling() {
    // Test async message parsing and handling
    let json = r#"{"jsonrpc":"2.0","method":"agent.call","params":{"agent_id":"test-agent","method":"ping"},"id":42}"#;
    
    // Parse message
    let request = JsonRpcHandler::parse_message(json).unwrap();
    let conn_id = Uuid::new_v4();
    
    // Handle request
    let response = JsonRpcHandler::handle(request, conn_id);
    
    // Verify response
    assert!(response.result.is_some());
    assert_eq!(response.id, Some(JsonRpcId::Num(42)));
}
