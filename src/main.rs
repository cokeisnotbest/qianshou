//! qianshou - High-performance WebSocket Relay Service
//! 
//! A local-agent-relay server supporting:
//! - WebSocket relay between clients and agents
//! - JSON-RPC protocol calls
//! - SSE log streaming
//! - Token authentication
//! - Heartbeat keep-alive (30 seconds)

use std::net::SocketAddr;
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

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            token: "default-token".to_string(),
        }
    }
}

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

    // Load configuration (using defaults for now)
    let config = Config::default();
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
