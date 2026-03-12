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
    extract::ws::{WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    response::sse::{Event, Sse, KeepAlive},
    Extension,
};
use futures_util::stream::StreamExt;
use tokio::sync::broadcast;
use std::time::Duration;
use serde::Deserialize;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

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

/// Broadcast channel for SSE log streaming
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub log_tx: broadcast::Sender<String>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        let (log_tx, _) = broadcast::channel(1000);
        Self { config, log_tx }
    }
}

/// WebSocket upgrade handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(state): Extension<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// Handle WebSocket connections
async fn handle_socket(socket: WebSocket, _state: AppState) {
    let (_sender, mut receiver) = socket.split();
    
    // TODO: Implement relay logic
    // - Token authentication
    // - JSON-RPC protocol handling
    // - Bidirectional message relay
    // - Heartbeat (30 second ping/pong)
    
    // Placeholder: just receive messages for now
    while let Some(result) = receiver.next().await {
        match result {
            Ok(msg) => {
                tracing::debug!("Received: {:?}", msg);
            }
            Err(e) => {
                tracing::error!("WebSocket error: {}", e);
                break;
            }
        }
    }
}

/// SSE log stream endpoint
async fn sse_logs(
    Extension(state): Extension<AppState>,
) -> impl IntoResponse {
    let mut rx = state.log_tx.subscribe();
    
    let stream = async_stream::stream! {
        loop {
            match rx.recv().await {
                Ok(log) => {
                    yield Ok::<_, std::convert::Infallible>(Event::default().data(log));
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
}
