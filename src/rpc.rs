//! JSON-RPC 2.0 Protocol Types
//!
//! Implementation of JSON-RPC 2.0 specification types.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC version constant
pub const JSONRPC_VERSION: &str = "2.0";

/// A JSON-RPC request object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct JsonRpcRequest {
    /// A unique identifier for the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<JsonRpcId>,
    
    /// JSON-RPC version - must be "2.0"
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    
    /// The name of the method to be invoked
    pub method: String,
    
    /// Parameters to be passed to the method
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request with the given method
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            id: None,
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params: None,
        }
    }
    
    /// Create a new JSON-RPC request with params
    pub fn with_params(method: impl Into<String>, params: Value) -> Self {
        Self {
            id: None,
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params: Some(params),
        }
    }
    
    /// Create a new JSON-RPC request with id and params
    pub fn with_id_and_params(id: JsonRpcId, method: impl Into<String>, params: Value) -> Self {
        Self {
            id: Some(id),
            jsonrpc: JSONRPC_VERSION.to_string(),
            method: method.into(),
            params: Some(params),
        }
    }
}

/// JSON-RPC identifier - can be string, number, or null
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum JsonRpcId {
    /// Numeric ID
    Num(i64),
    /// String ID
    Str(String),
    /// Null ID
    Null,
}

impl From<i64> for JsonRpcId {
    fn from(n: i64) -> Self {
        JsonRpcId::Num(n)
    }
}

impl From<String> for JsonRpcId {
    fn from(s: String) -> Self {
        JsonRpcId::Str(s)
    }
}

impl From<&str> for JsonRpcId {
    fn from(s: &str) -> Self {
        JsonRpcId::Str(s.to_string())
    }
}

/// A JSON-RPC response object (success case)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct JsonRpcResponse {
    /// JSON-RPC version - must be "2.0"
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    
    /// The result of the method invocation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    
    /// Error object if the call failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    
    /// The id of the request this response is for
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<JsonRpcId>,
}

impl JsonRpcResponse {
    /// Create a successful response
    pub fn success(id: Option<JsonRpcId>, result: Value) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }
    
    /// Create an error response
    pub fn error(id: Option<JsonRpcId>, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: JSONRPC_VERSION.to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

/// A JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    
    /// Error message
    pub message: String,
    
    /// Additional data about the error
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Create a new error with the given code and message
    pub fn new(code: JsonRpcErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.code(),
            message: message.into(),
            data: None,
        }
    }
    
    /// Create a new error with additional data
    pub fn with_data(code: JsonRpcErrorCode, message: impl Into<String>, data: Value) -> Self {
        Self {
            code: code.code(),
            message: message.into(),
            data: Some(data),
        }
    }
}

/// Standard JSON-RPC 2.0 error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonRpcErrorCode {
    /// Invalid JSON was received
    ParseError,
    /// The JSON sent is not a valid Request object
    InvalidRequest,
    /// The method does not exist / is not available
    MethodNotFound,
    /// Invalid method parameter(s)
    InvalidParams,
    /// Internal JSON-RPC error
    InternalError,
}

impl JsonRpcErrorCode {
    /// Get the error code value
    pub fn code(&self) -> i32 {
        match self {
            JsonRpcErrorCode::ParseError => -32700,
            JsonRpcErrorCode::InvalidRequest => -32600,
            JsonRpcErrorCode::MethodNotFound => -32601,
            JsonRpcErrorCode::InvalidParams => -32602,
            JsonRpcErrorCode::InternalError => -32603,
        }
    }
    
    /// Get the default message for this error code
    pub fn message(&self) -> &'static str {
        match self {
            JsonRpcErrorCode::ParseError => "Parse error",
            JsonRpcErrorCode::InvalidRequest => "Invalid Request",
            JsonRpcErrorCode::MethodNotFound => "Method not found",
            JsonRpcErrorCode::InvalidParams => "Invalid params",
            JsonRpcErrorCode::InternalError => "Internal error",
        }
    }
}

impl std::fmt::Display for JsonRpcErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message())
    }
}

impl std::error::Error for JsonRpcErrorCode {}

/// Helper to create a parse error response
pub fn parse_error(id: Option<JsonRpcId>) -> JsonRpcResponse {
    JsonRpcResponse::error(
        id,
        JsonRpcError::new(JsonRpcErrorCode::ParseError, "Invalid JSON"),
    )
}

/// Helper to create an invalid request error response
pub fn invalid_request(id: Option<JsonRpcId>) -> JsonRpcResponse {
    JsonRpcResponse::error(
        id,
        JsonRpcError::new(JsonRpcErrorCode::InvalidRequest, "The request is not a valid JSON-RPC 2.0 request"),
    )
}

/// Helper to create a method not found error response
pub fn method_not_found(id: Option<JsonRpcId>) -> JsonRpcResponse {
    JsonRpcResponse::error(
        id,
        JsonRpcError::new(JsonRpcErrorCode::MethodNotFound, "Method not found"),
    )
}

/// Helper to create an invalid params error response
pub fn invalid_params(id: Option<JsonRpcId>, message: &str) -> JsonRpcResponse {
    JsonRpcResponse::error(
        id,
        JsonRpcError::new(JsonRpcErrorCode::InvalidParams, message),
    )
}

/// Helper to create an internal error response
pub fn internal_error(id: Option<JsonRpcId>, message: &str) -> JsonRpcResponse {
    JsonRpcResponse::error(
        id,
        JsonRpcError::new(JsonRpcErrorCode::InternalError, message),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_serialization() {
        let req = JsonRpcRequest::new("test_method");
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""jsonrpc":"2.0""#));
        assert!(json.contains(r#""method":"test_method""#));
    }
    
    #[test]
    fn test_request_with_params() {
        let req = JsonRpcRequest::with_params("add", json!([1, 2]));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""method":"add""#));
        assert!(json.contains(r#"[1,2]"#));
    }
    
    #[test]
    fn test_request_with_id() {
        let req = JsonRpcRequest::with_id_and_params(
            JsonRpcId::Num(42),
            "test",
            json!({"key": "value"})
        );
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""id":42"#));
    }
    
    #[test]
    fn test_request_deserialization() {
        let json_str = r#"{"jsonrpc":"2.0","method":"test","params":[1,2],"id":1}"#;
        let req: JsonRpcRequest = serde_json::from_str(json_str).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "test");
        assert_eq!(req.params, Some(json!([1, 2])));
        assert_eq!(req.id, Some(JsonRpcId::Num(1)));
    }
    
    #[test]
    fn test_response_success() {
        let resp = JsonRpcResponse::success(Some(JsonRpcId::Num(1)), json!({"result": "ok"}));
        let json_str = serde_json::to_string(&resp).unwrap();
        assert!(json_str.contains(r#""jsonrpc":"2.0""#));
        assert!(json_str.contains(r#""result":{"result":"ok"}"#));
        assert!(json_str.contains(r#""id":1"#));
    }
    
    #[test]
    fn test_response_error() {
        let resp = JsonRpcResponse::error(
            Some(JsonRpcId::Num(1)),
            JsonRpcError::new(JsonRpcErrorCode::MethodNotFound, "Method not found")
        );
        let json_str = serde_json::to_string(&resp).unwrap();
        assert!(json_str.contains(r#""error":{"code":-32601,"message":"Method not found"}"#));
    }
    
    #[test]
    fn test_error_code_values() {
        assert_eq!(JsonRpcErrorCode::ParseError.code(), -32700);
        assert_eq!(JsonRpcErrorCode::InvalidRequest.code(), -32600);
        assert_eq!(JsonRpcErrorCode::MethodNotFound.code(), -32601);
        assert_eq!(JsonRpcErrorCode::InvalidParams.code(), -32602);
        assert_eq!(JsonRpcErrorCode::InternalError.code(), -32603);
    }
    
    #[test]
    fn test_error_code_messages() {
        assert_eq!(JsonRpcErrorCode::ParseError.message(), "Parse error");
        assert_eq!(JsonRpcErrorCode::MethodNotFound.message(), "Method not found");
    }
    
    #[test]
    fn test_json_rpc_id_variants() {
        let id_num = JsonRpcId::Num(42);
        let id_str = JsonRpcId::Str("test".to_string());
        let id_null = JsonRpcId::Null;
        
        assert_eq!(serde_json::to_string(&id_num).unwrap(), "42");
        assert!(serde_json::to_string(&id_str).unwrap().contains("test"));
        assert_eq!(serde_json::to_string(&id_null).unwrap(), "null");
    }
    
    #[test]
    fn test_helper_functions() {
        let resp = parse_error(Some(JsonRpcId::Num(1)));
        assert!(resp.error.is_some());
        assert_eq!(resp.error.as_ref().unwrap().code, -32700);
        
        let resp = method_not_found(Some(JsonRpcId::Num(2)));
        assert!(resp.error.is_some());
        assert_eq!(resp.error.as_ref().unwrap().code, -32601);
    }
    
    #[test]
    fn test_full_request_response_cycle() {
        // Simulate a full JSON-RPC request/response cycle
        let request_json = r#"{"jsonrpc":"2.0","method":"echo","params":["hello"],"id":1}"#;
        
        // Deserialize request
        let req: JsonRpcRequest = serde_json::from_str(request_json).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "echo");
        assert_eq!(req.params, Some(json!(["hello"])));
        
        // Create response
        let params = req.params.unwrap();
        let response = JsonRpcResponse::success(
            req.id,
            json!({"echoed": params})
        );
        
        // Verify response
        let response_json = serde_json::to_string(&response).unwrap();
        assert!(response_json.contains(r#""jsonrpc":"2.0""#));
        assert!(response_json.contains(r#""result":{"echoed":["hello"]}"#));
        assert!(response_json.contains(r#""id":1"#));
    }
}
