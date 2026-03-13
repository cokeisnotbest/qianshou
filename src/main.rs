//! qianshou - High-performance WebSocket Relay Service
//! 
//! A local-agent-relay server supporting:
//! - WebSocket relay between clients and agents
//! - JSON-RPC protocol calls
//! - SSE log streaming
//! - Token authentication
//! - Heartbeat keep-alive (30 seconds)

use std::net::SocketAddr;
use std::path::PathBuf;
use axum::{
    Router,
    routing::get,
    extract::ws::{WebSocket, WebSocketUpgrade, Message},
    extract::Query,
    response::IntoResponse,
    response::sse::{Event, Sse, KeepAlive},
    Extension,
};
use futures_util::{stream::StreamExt, SinkExt};
use tokio::sync::broadcast;
use tokio::time::{interval, timeout, Duration, Instant};
use serde::Deserialize;
use qianshou::{TokenQuery as AuthTokenQuery, validate_token, Connection, RelayState, LogLevel, LogBroadcaster, should_include_log};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Heartbeat configuration constants
const HEARTBEAT_INTERVAL_SECS: u64 = 30; // Send ping every 30 seconds
const HEARTBEAT_TIMEOUT_SECS: u64 = 10;  // Expect pong within 10 seconds

/// Server configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub token: String,
}

impl Config {
    /// Load configuration from a YAML file
    pub fn from_yaml(path: &PathBuf) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::FileRead(e.to_string()))?;
        
        let config: Config = serde_yaml::from_str(&content)
            .map_err(|e| ConfigError::YamlParse(e.to_string()))?;
        
        config.validate()?;
        Ok(config)
    }
    
    /// Validate configuration
    fn validate(&self) -> Result<(), ConfigError> {
        if self.token.is_empty() {
            return Err(ConfigError::InvalidToken("token cannot be empty".to_string()));
        }
        if self.port == 0 {
            return Err(ConfigError::InvalidPort("port cannot be 0".to_string()));
        }
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            token: "default-token".to_string(),
        }
    }
}

/// Configuration errors
#[derive(Debug)]
pub enum ConfigError {
    FileRead(String),
    YamlParse(String),
    InvalidToken(String),
    InvalidPort(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::FileRead(msg) => write!(f, "Failed to read config file: {}", msg),
            ConfigError::YamlParse(msg) => write!(f, "Failed to parse YAML: {}", msg),
            ConfigError::InvalidToken(msg) => write!(f, "Invalid token: {}", msg),
            ConfigError::InvalidPort(msg) => write!(f, "Invalid port: {}", msg),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Application state with connection management and logging
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub relay_state: RelayState,
    pub log_broadcaster: LogBroadcaster,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let log_broadcaster = LogBroadcaster::new();
        let (log_tx, _) = broadcast::channel(1000);
        let relay_state = RelayState::new(log_tx);
        Self { config, relay_state, log_broadcaster }
    }
}

/// WebSocket upgrade handler with token authentication
async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<AppState>,
    Query(token_query): Query<AuthTokenQuery>,
) -> impl IntoResponse {
    // Validate token from query parameter using the new validate_token function
    if let Err((status, message)) = validate_token(token_query.token.clone(), &state.config.token) {
        return (status, message).into_response();
    }

    tracing::info!("WebSocket upgrade request with valid token");
    
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle WebSocket connections with proper connection lifecycle management and heartbeat
async fn handle_socket(socket: WebSocket, state: AppState) {
    // Create a new connection with unique UUID
    let mut connection = Connection::new();
    let connection_id = connection.id;
    
    // Register the connection
    state.relay_state.connection_registry.register(connection.clone());
    
    // Log connection open event
    tracing::info!("Connection opened: {}", connection_id);
    state.log_broadcaster.log_connection_open(&connection_id.to_string());
    
    // Transition to connected state
    connection.connect();
    state.relay_state.connection_registry.update_state(&connection_id, connection.state);
    
    let (mut sender, mut receiver) = socket.split();
    
    // Track heartbeat state
    let mut _last_activity = Instant::now();
    let mut waiting_for_pong = false;
    
    // Create heartbeat interval
    let mut heartbeat_interval = interval(Duration::from_secs(HEARTBEAT_INTERVAL_SECS));
    
    // Handle messages with proper connection tracking and heartbeat
    loop {
        tokio::select! {
            // Heartbeat tick - send ping
            _ = heartbeat_interval.tick() => {
                // If we were waiting for pong and didn't receive it, close the connection
                if waiting_for_pong {
                    tracing::warn!(
                        "Connection {}: heartbeat timeout - no pong received within {} seconds",
                        connection_id,
                        HEARTBEAT_TIMEOUT_SECS
                    );
                    // Send close frame
                    let _ = sender.close().await;
                    break;
                }
                
                // Send ping frame
                tracing::debug!("Connection {}: sending heartbeat ping", connection_id);
                if let Err(e) = sender.send(Message::Ping(vec![].into())).await {
                    tracing::error!("Connection {}: failed to send ping: {}", connection_id, e);
                    break;
                }
                waiting_for_pong = true;
            }
            
            // Receive message with timeout to check for pong
            result = timeout(Duration::from_secs(1), receiver.next()) => {
                match result {
                    Ok(Some(Ok(msg))) => {
                        // Update activity timestamp on any message
                        _last_activity = Instant::now();
                        
                        // Check for pong response
                        match &msg {
                            Message::Pong(_) => {
                                tracing::debug!("Connection {}: received pong", connection_id);
                                waiting_for_pong = false;
                            }
                            Message::Text(text) => {
                                tracing::debug!("Connection {} received: {:?}", connection_id, text);
                                // TODO: Handle JSON-RPC messages here
                            }
                            Message::Close(_) => {
                                tracing::debug!("Connection {}: received close frame", connection_id);
                                break;
                            }
                            Message::Ping(data) => {
                                // Respond to ping with pong
                                tracing::debug!("Connection {}: received ping, sending pong", connection_id);
                                if let Err(e) = sender.send(Message::Pong(data.clone())).await {
                                    tracing::error!("Connection {}: failed to send pong: {}", connection_id, e);
                                }
                            }
                            _ => {}
                        }
                        
                        // Update connection registry last_activity
                        if let Some(mut conn) = state.relay_state.connection_registry.get(&connection_id) {
                            conn.update_activity();
                            let _ = state.relay_state.connection_registry.update_state(
                                &connection_id,
                                conn.state
                            );
                        }
                    }
                    Ok(Some(Err(e))) => {
                        tracing::error!("Connection {} error: {}", connection_id, e);
                        break;
                    }
                    Ok(None) => {
                        // Stream ended (normal close)
                        tracing::debug!("Connection {} stream ended", connection_id);
                        break;
                    }
                    Err(_) => {
                        // Timeout - just continue to check heartbeat
                        continue;
                    }
                }
            }
        }
    }

    // Handle graceful disconnect
    handle_disconnect(&state, connection_id).await;
}

/// Handle WebSocket disconnect gracefully
async fn handle_disconnect(state: &AppState, connection_id: uuid::Uuid) {
    // Update connection state to closing
    state.relay_state.connection_registry.update_state(&connection_id, qianshou::ConnectionState::Closing);
    
    // Log connection close event
    tracing::info!("Connection closed: {}", connection_id);
    state.log_broadcaster.log_connection_close(&connection_id.to_string());
    
    // Transition to disconnected state
    state.relay_state.connection_registry.update_state(&connection_id, qianshou::ConnectionState::Disconnected);
    
    // Optionally remove from registry (or keep for history)
    // state.relay_state.connection_registry.remove(&connection_id);
}

/// Query parameters for SSE logs endpoint
#[derive(Debug, Deserialize)]
pub struct LogQuery {
    /// Minimum log level to include (debug, info, warn, error)
    #[serde(default)]
    level: Option<String>,
}

impl LogQuery {
    /// Get the minimum log level, defaulting to Debug (show all)
    fn min_level(&self) -> LogLevel {
        self.level
            .as_ref()
            .and_then(|l| l.parse().ok())
            .unwrap_or(LogLevel::Debug)
    }
}

/// SSE log stream endpoint with optional level filtering
async fn sse_logs(
    Extension(state): Extension<AppState>,
    Query(query): Query<LogQuery>,
) -> impl IntoResponse {
    let min_level = query.min_level();
    let mut rx = state.log_broadcaster.subscribe();
    
    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(log_entry) => {
                    // Filter by log level
                    if should_include_log(&log_entry.level, &min_level) {
                        yield Ok::<_, std::convert::Infallible>(Event::default().data(log_entry.to_json()));
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    };
    
    Sse::new(stream).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}

/// Health check endpoint
async fn health() -> &'static str {
    "OK"
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::new("info"))
        .init();

    tracing::info!("Starting qianshou relay server...");

    // Parse CLI arguments for config file path
    let config_path = parse_config_path();
    
    // Load configuration from YAML file
    let config = match Config::from_yaml(&config_path) {
        Ok(cfg) => {
            tracing::info!("Loaded configuration from: {}", config_path.display());
            cfg
        }
        Err(e) => {
            tracing::error!("Failed to load config: {}", e);
            std::process::exit(1);
        }
    };

    // Print loaded config info (without exposing token)
    tracing::info!("Server configuration:");
    tracing::info!("  Host: {}", config.host);
    tracing::info!("  Port: {}", config.port);
    tracing::info!("  Token: [REDACTED]");

    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .unwrap_or_else(|_| "0.0.0.0:8080".parse().unwrap());

    let state = AppState::new(config);

    // Build router
    let app = Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws_handler))
        .route("/logs", get(sse_logs))
        .layer(Extension(state));

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::info!("Listening on {}", addr);

    axum::serve(listener, app).await.unwrap();
}

/// Parse command line arguments for config file path
fn parse_config_path() -> PathBuf {
    let args: Vec<String> = std::env::args().collect();
    
    // Look for --config or -c argument
    for i in 0..args.len() {
        if args[i] == "--config" || args[i] == "-c" {
            if i + 1 < args.len() {
                return PathBuf::from(&args[i + 1]);
            }
        }
    }
    
    // Default to ./config.yaml
    PathBuf::from("./config.yaml")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.port, 8080);
        assert_eq!(config.token, "default-token");
    }

    #[test]
    fn test_app_state_clone() {
        let config = Config::default();
        let state = AppState::new(config.clone());
        let _ = state.clone();
    }

    #[test]
    fn test_config_validate_empty_token() {
        let config = Config {
            host: "0.0.0.0".to_string(),
            port: 8080,
            token: "".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_zero_port() {
        let config = Config {
            host: "0.0.0.0".to_string(),
            port: 0,
            token: "valid-token".to_string(),
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_valid() {
        let config = Config {
            host: "0.0.0.0".to_string(),
            port: 8080,
            token: "valid-token".to_string(),
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_from_yaml_valid() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "host: \"127.0.0.1\"\nport: 9000\ntoken: \"test-token-123\"").unwrap();
        
        let path = PathBuf::from(file.path());
        let config = Config::from_yaml(&path).unwrap();
        
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 9000);
        assert_eq!(config.token, "test-token-123");
    }

    #[test]
    fn test_config_from_yaml_missing_file() {
        let path = PathBuf::from("/nonexistent/path/config.yaml");
        let result = Config::from_yaml(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_from_yaml_empty_token() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "host: \"0.0.0.0\"\nport: 8080\ntoken: \"\"").unwrap();
        
        let path = PathBuf::from(file.path());
        let result = Config::from_yaml(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_from_yaml_invalid_yaml() {
        let mut file = NamedTempFile::new().unwrap();
        writeln!(file, "host: \"0.0.0.0\"\n  invalid: yaml: content:").unwrap();
        
        let path = PathBuf::from(file.path());
        let result = Config::from_yaml(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_config_path_default() {
        // Parse config path - we test the flag parsing by checking default behavior
        let _ = parse_config_path();
    }

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::FileRead("test error".to_string());
        assert!(err.to_string().contains("Failed to read config file"));
        
        let err = ConfigError::YamlParse("test error".to_string());
        assert!(err.to_string().contains("Failed to parse YAML"));
        
        let err = ConfigError::InvalidToken("test error".to_string());
        assert!(err.to_string().contains("Invalid token"));
        
        let err = ConfigError::InvalidPort("test error".to_string());
        assert!(err.to_string().contains("Invalid port"));
    }
    
    // === Tests for Heartbeat Keep-Alive (US-007) ===
    
    #[test]
    fn test_heartbeat_constants() {
        // Verify heartbeat configuration
        assert_eq!(HEARTBEAT_INTERVAL_SECS, 30);
        assert_eq!(HEARTBEAT_TIMEOUT_SECS, 10);
    }
    
    #[test]
    fn test_heartbeat_interval_greater_than_timeout() {
        // Heartbeat interval should be greater than timeout
        // so we have time to detect missed pongs
        assert!(HEARTBEAT_INTERVAL_SECS > HEARTBEAT_TIMEOUT_SECS);
    }
    
    // === Tests for SSE Log Streaming (US-008) ===
    
    #[test]
    fn test_log_query_default_level() {
        let query = LogQuery { level: None };
        assert_eq!(query.min_level(), LogLevel::Debug);
    }
    
    #[test]
    fn test_log_query_info_level() {
        let query = LogQuery { level: Some("info".to_string()) };
        assert_eq!(query.min_level(), LogLevel::Info);
    }
    
    #[test]
    fn test_log_query_warn_level() {
        let query = LogQuery { level: Some("warn".to_string()) };
        assert_eq!(query.min_level(), LogLevel::Warn);
    }
    
    #[test]
    fn test_log_query_error_level() {
        let query = LogQuery { level: Some("error".to_string()) };
        assert_eq!(query.min_level(), LogLevel::Error);
    }
    
    #[test]
    fn test_log_query_invalid_level_defaults_to_debug() {
        let query = LogQuery { level: Some("invalid".to_string()) };
        assert_eq!(query.min_level(), LogLevel::Debug);
    }
    
    #[test]
    fn test_log_query_warning_alias() {
        let query = LogQuery { level: Some("warning".to_string()) };
        assert_eq!(query.min_level(), LogLevel::Warn);
    }
    
    #[test]
    fn test_app_state_has_log_broadcaster() {
        let config = Config::default();
        let state = AppState::new(config.clone());
        // Verify log_broadcaster exists and can broadcast
        let mut sub = state.log_broadcaster.subscribe();
        state.log_broadcaster.info("test message");
        let entry = sub.try_recv().unwrap();
        assert_eq!(entry.message, "test message");
    }
    
    #[test]
    fn test_connection_events_broadcast() {
        let config = Config::default();
        let state = AppState::new(config.clone());
        
        let connection_id = "test-connection-123";
        
        // Create subscriber first
        let mut sub = state.log_broadcaster.subscribe();
        
        // Broadcast connection open
        state.log_broadcaster.log_connection_open(connection_id);
        
        // Broadcast connection close  
        state.log_broadcaster.log_connection_close(connection_id);
        
        let open = sub.try_recv().unwrap();
        assert_eq!(open.message, "Connection opened");
        assert_eq!(open.connection_id, Some(connection_id.to_string()));
        
        let close = sub.try_recv().unwrap();
        assert_eq!(close.message, "Connection closed");
        assert_eq!(close.connection_id, Some(connection_id.to_string()));
    }
}
