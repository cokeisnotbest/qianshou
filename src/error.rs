//! Comprehensive Error Handling Module
//!
//! Provides unified error types for the application with proper logging,
//! display formatting, and HTTP/WebSocket response conversion.

use axum::response::{IntoResponse, Response};
use hyper::StatusCode;
use std::fmt;

/// Application-wide error type
#[derive(Debug)]
pub enum AppError {
    /// Configuration-related errors
    Config(ConfigError),
    /// Authentication errors
    Auth(AuthError),
    /// JSON-RPC protocol errors
    Rpc(RpcError),
    /// Connection-related errors
    Connection(ConnectionError),
    /// Internal server errors
    Internal(InternalError),
}

/// Configuration errors
#[derive(Debug, Clone)]
pub enum ConfigError {
    FileRead(String),
    YamlParse(String),
    InvalidToken(String),
    InvalidPort(String),
}

/// Authentication errors
#[derive(Debug, Clone)]
pub enum AuthError {
    MissingToken,
    InvalidToken,
    TokenExpired,
}

/// JSON-RPC errors
#[derive(Debug, Clone)]
pub enum RpcError {
    ParseError(String),
    InvalidRequest(String),
    MethodNotFound(String),
    InvalidParams(String),
    InternalError(String),
}

/// Connection errors
#[derive(Debug, Clone)]
pub enum ConnectionError {
    NotFound(String),
    AlreadyConnected(String),
    NotConnected(String),
    RemoteNotFound(String),
    SendFailed(String),
    InvalidState(String),
}

/// Internal server errors
#[derive(Debug, Clone)]
pub enum InternalError {
    Unknown(String),
    TaskPanic(String),
}

// ============================================================================
// Display implementations
// ============================================================================

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Config(e) => write!(f, "Config error: {}", e),
            AppError::Auth(e) => write!(f, "Auth error: {}", e),
            AppError::Rpc(e) => write!(f, "RPC error: {}", e),
            AppError::Connection(e) => write!(f, "Connection error: {}", e),
            AppError::Internal(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::FileRead(msg) => write!(f, "Failed to read config file: {}", msg),
            ConfigError::YamlParse(msg) => write!(f, "Failed to parse YAML: {}", msg),
            ConfigError::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
            ConfigError::InvalidPort(msg) => write!(f, "Invalid port: {}", msg),
        }
    }
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::MissingToken => write!(f, "Missing authentication token"),
            AuthError::InvalidToken => write!(f, "Invalid authentication token"),
            AuthError::TokenExpired => write!(f, "Authentication token has expired"),
        }
    }
}

impl fmt::Display for RpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RpcError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            RpcError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            RpcError::MethodNotFound(msg) => write!(f, "Method not found: {}", msg),
            RpcError::InvalidParams(msg) => write!(f, "Invalid params: {}", msg),
            RpcError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl fmt::Display for ConnectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionError::NotFound(id) => write!(f, "Connection not found: {}", id),
            ConnectionError::AlreadyConnected(id) => write!(f, "Connection already exists: {}", id),
            ConnectionError::NotConnected(id) => write!(f, "Not connected: {}", id),
            ConnectionError::RemoteNotFound(id) => write!(f, "Remote connection not found: {}", id),
            ConnectionError::SendFailed(msg) => write!(f, "Send failed: {}", msg),
            ConnectionError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
        }
    }
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InternalError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
            InternalError::TaskPanic(msg) => write!(f, "Task panic: {}", msg),
        }
    }
}

// ============================================================================
// std::error::Error implementations
// ============================================================================

impl std::error::Error for AppError {}
impl std::error::Error for ConfigError {}
impl std::error::Error for AuthError {}
impl std::error::Error for RpcError {}
impl std::error::Error for ConnectionError {}
impl std::error::Error for InternalError {}

// ============================================================================
// From trait implementations for error conversion
// ============================================================================

// ConfigError -> AppError
impl From<ConfigError> for AppError {
    fn from(err: ConfigError) -> Self {
        AppError::Config(err)
    }
}

// AuthError -> AppError
impl From<AuthError> for AppError {
    fn from(err: AuthError) -> Self {
        AppError::Auth(err)
    }
}

// RpcError -> AppError
impl From<RpcError> for AppError {
    fn from(err: RpcError) -> Self {
        AppError::Rpc(err)
    }
}

// ConnectionError -> AppError
impl From<ConnectionError> for AppError {
    fn from(err: ConnectionError) -> Self {
        AppError::Connection(err)
    }
}

// InternalError -> AppError
impl From<InternalError> for AppError {
    fn from(err: InternalError) -> Self {
        AppError::Internal(err)
    }
}

// String -> ConfigError
impl From<String> for ConfigError {
    fn from(s: String) -> Self {
        ConfigError::FileRead(s)
    }
}

// String -> AuthError
impl From<String> for AuthError {
    fn from(s: String) -> Self {
        match s.as_str() {
            "missing" => AuthError::MissingToken,
            "invalid" => AuthError::InvalidToken,
            "expired" => AuthError::TokenExpired,
            _ => AuthError::InvalidToken,
        }
    }
}

// String -> RpcError
impl From<String> for RpcError {
    fn from(s: String) -> Self {
        RpcError::InternalError(s)
    }
}

// String -> ConnectionError
impl From<String> for ConnectionError {
    fn from(s: String) -> Self {
        ConnectionError::InvalidState(s)
    }
}

// String -> InternalError
impl From<String> for InternalError {
    fn from(s: String) -> Self {
        InternalError::Unknown(s)
    }
}

// ============================================================================
// IntoResponse implementations for HTTP responses
// ============================================================================

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // Log the error with appropriate level
        tracing::error!("Application error: {}", self);
        
        let (status, message) = match &self {
            AppError::Config(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
            AppError::Auth(_) => (StatusCode::UNAUTHORIZED, self.to_string()),
            AppError::Rpc(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            AppError::Connection(_) => (StatusCode::SERVICE_UNAVAILABLE, self.to_string()),
            AppError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()),
        };
        
        (status, message).into_response()
    }
}

impl IntoResponse for ConfigError {
    fn into_response(self) -> Response {
        tracing::error!("Config error: {}", self);
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        tracing::warn!("Auth error: {}", self);
        (StatusCode::UNAUTHORIZED, self.to_string()).into_response()
    }
}

impl IntoResponse for RpcError {
    fn into_response(self) -> Response {
        tracing::warn!("RPC error: {}", self);
        
        // Convert to JSON-RPC error response
        let (code, message) = match &self {
            RpcError::ParseError(_) => (-32700, "Parse error"),
            RpcError::InvalidRequest(_) => (-32600, "Invalid Request"),
            RpcError::MethodNotFound(_) => (-32601, "Method not found"),
            RpcError::InvalidParams(_) => (-32602, "Invalid params"),
            RpcError::InternalError(_) => (-32603, "Internal error"),
        };
        
        let error_obj = serde_json::json!({
            "jsonrpc": "2.0",
            "error": {
                "code": code,
                "message": message,
                "data": self.to_string()
            },
            "id": null
        });
        
        let body = serde_json::to_string(&error_obj).unwrap_or_default();
        
        let mut response = (StatusCode::OK, body).into_response();
        response.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            hyper::header::HeaderValue::from_static("application/json"),
        );
        response
    }
}

impl IntoResponse for ConnectionError {
    fn into_response(self) -> Response {
        tracing::error!("Connection error: {}", self);
        
        let error_obj = serde_json::json!({
            "jsonrpc": "2.0",
            "error": {
                "code": -32000,
                "message": "Connection error",
                "data": self.to_string()
            },
            "id": null
        });
        
        let body = serde_json::to_string(&error_obj).unwrap_or_default();
        
        let mut response = (StatusCode::OK, body).into_response();
        response.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            hyper::header::HeaderValue::from_static("application/json"),
        );
        response
    }
}

impl IntoResponse for InternalError {
    fn into_response(self) -> Response {
        tracing::error!("Internal error: {}", self);
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}

// ============================================================================
// Helper functions for creating errors
// ============================================================================

/// Create a config not found error
pub fn config_not_found(path: &str) -> ConfigError {
    ConfigError::FileRead(format!("Config file not found: {}", path))
}

/// Create an invalid token error
#[allow(dead_code)]
pub fn invalid_token(_expected: &str) -> AuthError {
    AuthError::InvalidToken
}

/// Create a connection not found error
pub fn connection_not_found(id: &str) -> ConnectionError {
    ConnectionError::NotFound(id.to_string())
}

/// Create a remote not found error
pub fn remote_not_found(id: &str) -> ConnectionError {
    ConnectionError::RemoteNotFound(id.to_string())
}

/// Create an internal error
pub fn internal_error(msg: &str) -> InternalError {
    InternalError::Unknown(msg.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test AppError display
    #[test]
    fn test_app_error_display() {
        let err: AppError = ConfigError::InvalidToken("test".to_string()).into();
        assert_eq!(err.to_string(), "Config error: Invalid token: test");
        
        let err: AppError = AuthError::MissingToken.into();
        assert_eq!(err.to_string(), "Auth error: Missing authentication token");
        
        let err: AppError = RpcError::MethodNotFound("test".to_string()).into();
        assert_eq!(err.to_string(), "RPC error: Method not found: test");
        
        let err: AppError = ConnectionError::NotFound("abc".to_string()).into();
        assert_eq!(err.to_string(), "Connection error: Connection not found: abc");
        
        let err: AppError = InternalError::Unknown("test".to_string()).into();
        assert_eq!(err.to_string(), "Internal error: Unknown error: test");
    }

    // Test ConfigError display
    #[test]
    fn test_config_error_display() {
        assert_eq!(
            ConfigError::FileRead("test".to_string()).to_string(),
            "Failed to read config file: test"
        );
        assert_eq!(
            ConfigError::YamlParse("test".to_string()).to_string(),
            "Failed to parse YAML: test"
        );
        assert_eq!(
            ConfigError::InvalidToken("test".to_string()).to_string(),
            "Invalid token: test"
        );
        assert_eq!(
            ConfigError::InvalidPort("test".to_string()).to_string(),
            "Invalid port: test"
        );
    }

    // Test AuthError display
    #[test]
    fn test_auth_error_display() {
        assert_eq!(AuthError::MissingToken.to_string(), "Missing authentication token");
        assert_eq!(AuthError::InvalidToken.to_string(), "Invalid authentication token");
        assert_eq!(AuthError::TokenExpired.to_string(), "Authentication token has expired");
    }

    // Test RpcError display
    #[test]
    fn test_rpc_error_display() {
        assert_eq!(
            RpcError::ParseError("test".to_string()).to_string(),
            "Parse error: test"
        );
        assert_eq!(
            RpcError::InvalidRequest("test".to_string()).to_string(),
            "Invalid request: test"
        );
        assert_eq!(
            RpcError::MethodNotFound("test".to_string()).to_string(),
            "Method not found: test"
        );
        assert_eq!(
            RpcError::InvalidParams("test".to_string()).to_string(),
            "Invalid params: test"
        );
        assert_eq!(
            RpcError::InternalError("test".to_string()).to_string(),
            "Internal error: test"
        );
    }

    // Test ConnectionError display
    #[test]
    fn test_connection_error_display() {
        assert_eq!(
            ConnectionError::NotFound("abc".to_string()).to_string(),
            "Connection not found: abc"
        );
        assert_eq!(
            ConnectionError::AlreadyConnected("abc".to_string()).to_string(),
            "Connection already exists: abc"
        );
        assert_eq!(
            ConnectionError::NotConnected("abc".to_string()).to_string(),
            "Not connected: abc"
        );
        assert_eq!(
            ConnectionError::RemoteNotFound("abc".to_string()).to_string(),
            "Remote connection not found: abc"
        );
        assert_eq!(
            ConnectionError::SendFailed("test".to_string()).to_string(),
            "Send failed: test"
        );
        assert_eq!(
            ConnectionError::InvalidState("test".to_string()).to_string(),
            "Invalid state: test"
        );
    }

    // Test InternalError display
    #[test]
    fn test_internal_error_display() {
        assert_eq!(
            InternalError::Unknown("test".to_string()).to_string(),
            "Unknown error: test"
        );
        assert_eq!(
            InternalError::TaskPanic("test".to_string()).to_string(),
            "Task panic: test"
        );
    }

    // Test From trait implementations
    #[test]
    fn test_from_config_error() {
        let err: AppError = ConfigError::FileRead("test".to_string()).into();
        assert!(matches!(err, AppError::Config(ConfigError::FileRead(_))));
    }

    #[test]
    fn test_from_auth_error() {
        let err: AppError = AuthError::MissingToken.into();
        assert!(matches!(err, AppError::Auth(AuthError::MissingToken)));
    }

    #[test]
    fn test_from_rpc_error() {
        let err: AppError = RpcError::MethodNotFound("test".to_string()).into();
        assert!(matches!(err, AppError::Rpc(RpcError::MethodNotFound(_))));
    }

    #[test]
    fn test_from_connection_error() {
        let err: AppError = ConnectionError::NotFound("test".to_string()).into();
        assert!(matches!(err, AppError::Connection(ConnectionError::NotFound(_))));
    }

    #[test]
    fn test_from_internal_error() {
        let err: AppError = InternalError::Unknown("test".to_string()).into();
        assert!(matches!(err, AppError::Internal(InternalError::Unknown(_))));
    }

    // Test string conversions
    #[test]
    fn test_string_to_config_error() {
        let err: ConfigError = "test error".to_string().into();
        assert!(matches!(err, ConfigError::FileRead(_)));
    }

    #[test]
    fn test_string_to_auth_error() {
        let err: AuthError = "missing".to_string().into();
        assert!(matches!(err, AuthError::MissingToken));
        
        let err: AuthError = "invalid".to_string().into();
        assert!(matches!(err, AuthError::InvalidToken));
        
        let err: AuthError = "expired".to_string().into();
        assert!(matches!(err, AuthError::TokenExpired));
    }

    #[test]
    fn test_string_to_rpc_error() {
        let err: RpcError = "test".to_string().into();
        assert!(matches!(err, RpcError::InternalError(_)));
    }

    #[test]
    fn test_string_to_connection_error() {
        let err: ConnectionError = "test".to_string().into();
        assert!(matches!(err, ConnectionError::InvalidState(_)));
    }

    #[test]
    fn test_string_to_internal_error() {
        let err: InternalError = "test".to_string().into();
        assert!(matches!(err, InternalError::Unknown(_)));
    }

    // Test helper functions
    #[test]
    fn test_helper_functions() {
        assert_eq!(config_not_found("test.yaml").to_string(), "Failed to read config file: Config file not found: test.yaml");
        assert_eq!(connection_not_found("abc").to_string(), "Connection not found: abc");
        assert_eq!(remote_not_found("abc").to_string(), "Remote connection not found: abc");
        assert_eq!(internal_error("oops").to_string(), "Unknown error: oops");
    }

    // Test error type inference
    #[test]
    fn test_error_type_inference() {
        // Test that ? operator works with these error types
        fn return_config_error() -> Result<(), ConfigError> {
            Ok(())
        }
        
        fn return_auth_error() -> Result<(), AuthError> {
            Ok(())
        }
        
        fn return_rpc_error() -> Result<(), RpcError> {
            Ok(())
        }
        
        fn return_connection_error() -> Result<(), ConnectionError> {
            Ok(())
        }
        
        fn return_internal_error() -> Result<(), InternalError> {
            Ok(())
        }
        
        // These should compile
        let _: Result<(), AppError> = return_config_error().map_err(AppError::from);
        let _: Result<(), AppError> = return_auth_error().map_err(AppError::from);
        let _: Result<(), AppError> = return_rpc_error().map_err(AppError::from);
        let _: Result<(), AppError> = return_connection_error().map_err(AppError::from);
        let _: Result<(), AppError> = return_internal_error().map_err(AppError::from);
    }
}
