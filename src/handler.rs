//! JSON-RPC Request Handler
//!
//! Handles incoming JSON-RPC requests from WebSocket connections.

use serde_json::{json, Value};
use super::{
    JsonRpcRequest, JsonRpcResponse,
    parse_error, invalid_request, method_not_found, invalid_params,
    ConnectionId,
};

/// Result type for handler operations
pub type HandlerResult = Result<Value, HandlerError>;

/// Handler error
#[derive(Debug, Clone)]
pub struct HandlerError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

impl HandlerError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
}

/// JSON-RPC Request Handler
///
/// Routes JSON-RPC requests to appropriate handlers based on method name.
pub struct JsonRpcHandler;

impl JsonRpcHandler {
    /// Handle a JSON-RPC request
    pub fn handle(request: JsonRpcRequest, connection_id: ConnectionId) -> JsonRpcResponse {
        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return invalid_request(request.id);
        }

        // Route to appropriate handler
        match request.method.as_str() {
            "agent.connect" => Self::handle_agent_connect(request, connection_id),
            "agent.disconnect" => Self::handle_agent_disconnect(request, connection_id),
            "agent.call" => Self::handle_agent_call(request, connection_id),
            "relay.forward" => Self::handle_relay_forward(request, connection_id),
            _ => method_not_found(request.id),
        }
    }

    /// Parse a text message as JSON-RPC request
    pub fn parse_message(message: &str) -> Result<JsonRpcRequest, JsonRpcResponse> {
        // Try to parse as JSON
        let parsed: Result<Value, _> = serde_json::from_str(message);
        
        match parsed {
            Err(_) => {
                // Invalid JSON - return parse error (without id since we can't parse it)
                Err(parse_error(None))
            }
            Ok(value) => {
                // Try to parse as JsonRpcRequest
                match serde_json::from_value::<JsonRpcRequest>(value) {
                    Err(_) => {
                        // Valid JSON but not a valid JSON-RPC request
                        Err(invalid_request(None))
                    }
                    Ok(request) => {
                        Ok(request)
                    }
                }
            }
        }
    }

    /// Handle agent.connect method
    /// 
    /// Registers an agent with the relay.
    /// 
    /// Parameters (optional):
    /// - agent_id: String - unique identifier for the agent
    /// - capabilities: Array - list of agent capabilities
    fn handle_agent_connect(request: JsonRpcRequest, connection_id: ConnectionId) -> JsonRpcResponse {
        // Extract parameters
        let params = request.params.unwrap_or(Value::Null);
        
        // Parse agent_id from params
        let agent_id = params.get("agent_id")
            .and_then(|v: &Value| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| format!("agent-{}", connection_id));
        
        // Parse capabilities from params
        let capabilities: Vec<String> = params.get("capabilities")
            .and_then(|v: &Value| v.as_array())
            .map(|arr: &Vec<Value>| {
                arr.iter()
                    .filter_map(|v: &Value| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        tracing::info!("Agent connected: {} (id: {}, capabilities: {:?})", 
            agent_id, connection_id, capabilities);

        JsonRpcResponse::success(
            request.id,
            json!({
                "status": "connected",
                "agent_id": agent_id,
                "connection_id": connection_id.to_string(),
                "capabilities": capabilities
            }),
        )
    }

    /// Handle agent.disconnect method
    /// 
    /// Disconnects an agent from the relay.
    /// 
    /// Parameters (optional):
    /// - agent_id: String - unique identifier for the agent
    fn handle_agent_disconnect(request: JsonRpcRequest, connection_id: ConnectionId) -> JsonRpcResponse {
        // Extract parameters
        let params = request.params.unwrap_or(Value::Null);
        
        // Parse agent_id from params
        let agent_id = params.get("agent_id")
            .and_then(|v: &Value| v.as_str())
            .map(String::from)
            .unwrap_or_else(|| format!("agent-{}", connection_id));

        tracing::info!("Agent disconnected: {} (connection: {})", agent_id, connection_id);

        JsonRpcResponse::success(
            request.id,
            json!({
                "status": "disconnected",
                "agent_id": agent_id,
                "connection_id": connection_id.to_string()
            }),
        )
    }

    /// Handle agent.call method
    /// 
    /// Invokes a method on an agent.
    /// 
    /// Parameters:
    /// - agent_id: String - target agent identifier
    /// - method: String - method name to invoke
    /// - params: Any - parameters for the method (optional)
    fn handle_agent_call(request: JsonRpcRequest, _connection_id: ConnectionId) -> JsonRpcResponse {
        // Extract parameters
        let params = request.params.unwrap_or(Value::Null);
        
        // Parse target agent
        let agent_id = match params.get("agent_id").and_then(|v: &Value| v.as_str()) {
            Some(id) => id,
            None => {
                return invalid_params(request.id, "Missing required parameter: agent_id");
            }
        };
        
        // Parse method to invoke
        let method = match params.get("method").and_then(|v: &Value| v.as_str()) {
            Some(m) => m,
            None => {
                return invalid_params(request.id, "Missing required parameter: method");
            }
        };
        
        // Get method parameters (optional)
        let _method_params = params.get("params").cloned().unwrap_or(Value::Null);

        tracing::info!("Agent call: {}.{}()", agent_id, method);

        // In a real implementation, this would forward to the target agent
        // For now, return a mock response
        JsonRpcResponse::success(
            request.id,
            json!({
                "agent_id": agent_id,
                "method": method,
                "result": {
                    "status": "executed",
                    "message": format!("Method {} called on agent {}", method, agent_id)
                }
            }),
        )
    }

    /// Handle relay.forward method
    /// 
    /// Forwards a message to another connection through the relay.
    /// 
    /// Parameters:
    /// - target_id: String - target connection ID
    /// - message: Any - message to forward
    fn handle_relay_forward(request: JsonRpcRequest, connection_id: ConnectionId) -> JsonRpcResponse {
        // Extract parameters
        let params = request.params.unwrap_or(Value::Null);
        
        // Parse target connection
        let target_id = match params.get("target_id").and_then(|v: &Value| v.as_str()) {
            Some(id) => id,
            None => {
                return invalid_params(request.id, "Missing required parameter: target_id");
            }
        };
        
        // Parse message to forward
        let message = params.get("message").cloned().unwrap_or(Value::Null);

        tracing::debug!("Relaying message from {} to {}", connection_id, target_id);

        // In a real implementation, this would forward to the target connection
        // For now, return a mock response
        JsonRpcResponse::success(
            request.id,
            json!({
                "status": "forwarded",
                "from": connection_id.to_string(),
                "target": target_id,
                "message": message
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::JsonRpcId;
    use uuid::Uuid;

    fn test_connection_id() -> ConnectionId {
        Uuid::new_v4()
    }

    #[test]
    fn test_parse_message_valid_json_rpc() {
        let json = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
        let result = JsonRpcHandler::parse_message(json);
        assert!(result.is_ok());
        let req = result.unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "test");
        assert_eq!(req.id, Some(JsonRpcId::Num(1)));
    }

    #[test]
    fn test_parse_message_with_params() {
        let json = r#"{"jsonrpc":"2.0","method":"test","params":{"key":"value"},"id":1}"#;
        let result = JsonRpcHandler::parse_message(json);
        assert!(result.is_ok());
        let req = result.unwrap();
        assert_eq!(req.method, "test");
        assert!(req.params.is_some());
    }

    #[test]
    fn test_parse_message_invalid_json() {
        let json = "not valid json";
        let result = JsonRpcHandler::parse_message(json);
        assert!(result.is_err());
        let resp = result.unwrap_err();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32700);
    }

    #[test]
    fn test_parse_message_valid_json_not_rpc() {
        let json = r#"{"foo":"bar"}"#;
        let result = JsonRpcHandler::parse_message(json);
        assert!(result.is_err());
        let resp = result.unwrap_err();
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32600);
    }

    #[test]
    fn test_handle_valid_version() {
        let request = JsonRpcRequest {
            id: Some(JsonRpcId::Num(1)),
            jsonrpc: "2.0".to_string(),
            method: "test".to_string(),
            params: None,
        };
        let conn_id = test_connection_id();
        let response = JsonRpcHandler::handle(request, conn_id);
        // Should route to method_not_found since "test" is not a valid method
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32601);
    }

    #[test]
    fn test_handle_invalid_version() {
        let request = JsonRpcRequest {
            id: Some(JsonRpcId::Num(1)),
            jsonrpc: "1.0".to_string(),
            method: "test".to_string(),
            params: None,
        };
        let conn_id = test_connection_id();
        let response = JsonRpcHandler::handle(request, conn_id);
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32600);
    }

    #[test]
    fn test_handle_agent_connect() {
        let request = JsonRpcRequest {
            id: Some(JsonRpcId::Num(1)),
            jsonrpc: "2.0".to_string(),
            method: "agent.connect".to_string(),
            params: Some(json!({"agent_id": "test-agent", "capabilities": ["chat", "code"]})),
        };
        let conn_id = test_connection_id();
        let response = JsonRpcHandler::handle(request, conn_id);
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert_eq!(result.get("status").unwrap(), "connected");
        assert_eq!(result.get("agent_id").unwrap(), "test-agent");
    }

    #[test]
    fn test_handle_agent_connect_no_params() {
        let request = JsonRpcRequest {
            id: Some(JsonRpcId::Num(2)),
            jsonrpc: "2.0".to_string(),
            method: "agent.connect".to_string(),
            params: None,
        };
        let conn_id = test_connection_id();
        let response = JsonRpcHandler::handle(request, conn_id);
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert_eq!(result.get("status").unwrap(), "connected");
    }

    #[test]
    fn test_handle_agent_disconnect() {
        let request = JsonRpcRequest {
            id: Some(JsonRpcId::Num(1)),
            jsonrpc: "2.0".to_string(),
            method: "agent.disconnect".to_string(),
            params: Some(json!({"agent_id": "test-agent"})),
        };
        let conn_id = test_connection_id();
        let response = JsonRpcHandler::handle(request, conn_id);
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert_eq!(result.get("status").unwrap(), "disconnected");
    }

    #[test]
    fn test_handle_agent_call() {
        let request = JsonRpcRequest {
            id: Some(JsonRpcId::Num(1)),
            jsonrpc: "2.0".to_string(),
            method: "agent.call".to_string(),
            params: Some(json!({
                "agent_id": "target-agent",
                "method": "execute",
                "params": {"command": "test"}
            })),
        };
        let conn_id = test_connection_id();
        let response = JsonRpcHandler::handle(request, conn_id);
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result.get("result").is_some());
    }

    #[test]
    fn test_handle_agent_call_missing_agent_id() {
        let request = JsonRpcRequest {
            id: Some(JsonRpcId::Num(1)),
            jsonrpc: "2.0".to_string(),
            method: "agent.call".to_string(),
            params: Some(json!({"method": "execute"})),
        };
        let conn_id = test_connection_id();
        let response = JsonRpcHandler::handle(request, conn_id);
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32602);
    }

    #[test]
    fn test_handle_agent_call_missing_method() {
        let request = JsonRpcRequest {
            id: Some(JsonRpcId::Num(1)),
            jsonrpc: "2.0".to_string(),
            method: "agent.call".to_string(),
            params: Some(json!({"agent_id": "target-agent"})),
        };
        let conn_id = test_connection_id();
        let response = JsonRpcHandler::handle(request, conn_id);
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32602);
    }

    #[test]
    fn test_handle_relay_forward() {
        let request = JsonRpcRequest {
            id: Some(JsonRpcId::Num(1)),
            jsonrpc: "2.0".to_string(),
            method: "relay.forward".to_string(),
            params: Some(json!({
                "target_id": "target-connection",
                "message": {"data": "hello"}
            })),
        };
        let conn_id = test_connection_id();
        let response = JsonRpcHandler::handle(request, conn_id);
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert_eq!(result.get("status").unwrap(), "forwarded");
    }

    #[test]
    fn test_handle_relay_forward_missing_target() {
        let request = JsonRpcRequest {
            id: Some(JsonRpcId::Num(1)),
            jsonrpc: "2.0".to_string(),
            method: "relay.forward".to_string(),
            params: Some(json!({"message": "test"})),
        };
        let conn_id = test_connection_id();
        let response = JsonRpcHandler::handle(request, conn_id);
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32602);
    }

    #[test]
    fn test_handle_unknown_method() {
        let request = JsonRpcRequest {
            id: Some(JsonRpcId::Num(1)),
            jsonrpc: "2.0".to_string(),
            method: "unknown.method".to_string(),
            params: None,
        };
        let conn_id = test_connection_id();
        let response = JsonRpcHandler::handle(request, conn_id);
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32601);
    }

    #[test]
    fn test_response_preserves_id() {
        let request = JsonRpcRequest {
            id: Some(JsonRpcId::Str("request-123".to_string())),
            jsonrpc: "2.0".to_string(),
            method: "agent.connect".to_string(),
            params: None,
        };
        let conn_id = test_connection_id();
        let response = JsonRpcHandler::handle(request, conn_id);
        assert_eq!(response.id, Some(JsonRpcId::Str("request-123".to_string())));
    }

    #[test]
    fn test_response_preserves_numeric_id() {
        let request = JsonRpcRequest {
            id: Some(JsonRpcId::Num(42)),
            jsonrpc: "2.0".to_string(),
            method: "agent.connect".to_string(),
            params: None,
        };
        let conn_id = test_connection_id();
        let response = JsonRpcHandler::handle(request, conn_id);
        assert_eq!(response.id, Some(JsonRpcId::Num(42)));
    }
}
