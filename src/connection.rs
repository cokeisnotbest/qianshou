//! WebSocket Connection Handler
//!
//! Provides connection state management, unique connection IDs,
//! and proper connection lifecycle handling.

use chrono::Utc;
use hyper::StatusCode;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;
use parking_lot::RwLock;
use std::collections::HashMap;

/// Unique connection identifier
pub type ConnectionId = Uuid;

/// Connection state
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionState {
    /// Connection is being established
    Connecting,
    /// Connection is active and ready
    Connected,
    /// Connection is closing
    Closing,
    /// Connection is closed
    Disconnected,
}

impl Default for ConnectionState {
    fn default() -> Self {
        ConnectionState::Connecting
    }
}

/// WebSocket connection information
#[derive(Debug, Clone)]
pub struct Connection {
    pub id: ConnectionId,
    pub state: ConnectionState,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<Utc>,
}

impl Connection {
    /// Create a new connection with a unique ID
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            state: ConnectionState::Connecting,
            created_at: now,
            last_activity: now,
        }
    }

    /// Transition to connected state
    pub fn connect(&mut self) {
        self.state = ConnectionState::Connected;
        self.last_activity = Utc::now();
    }

    /// Transition to closing state
    pub fn close(&mut self) {
        self.state = ConnectionState::Closing;
        self.last_activity = Utc::now();
    }

    /// Transition to disconnected state
    pub fn disconnect(&mut self) {
        self.state = ConnectionState::Disconnected;
        self.last_activity = Utc::now();
    }

    /// Update last activity timestamp
    pub fn update_activity(&mut self) {
        self.last_activity = Utc::now();
    }
}

/// Connection registry for tracking all active connections
#[derive(Debug)]
pub struct ConnectionRegistry {
    connections: RwLock<HashMap<ConnectionId, Connection>>,
}

impl Default for ConnectionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionRegistry {
    /// Create a new connection registry
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new connection
    pub fn register(&self, connection: Connection) -> ConnectionId {
        let id = connection.id;
        let mut connections = self.connections.write();
        connections.insert(id, connection);
        id
    }

    /// Get a connection by ID
    pub fn get(&self, id: &ConnectionId) -> Option<Connection> {
        let connections = self.connections.read();
        connections.get(id).cloned()
    }

    /// Get a mutable reference to a connection by ID
    #[allow(dead_code)]
    pub fn get_mut(&self, _id: &ConnectionId) -> Option<parking_lot::RwLockWriteGuard<'_, HashMap<ConnectionId, Connection>>> {
        Some(self.connections.write())
    }

    /// Update connection state
    pub fn update_state(&self, id: &ConnectionId, state: ConnectionState) -> bool {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.get_mut(id) {
            conn.state = state;
            conn.last_activity = Utc::now();
            true
        } else {
            false
        }
    }

    /// Remove a connection
    pub fn remove(&self, id: &ConnectionId) -> Option<Connection> {
        let mut connections = self.connections.write();
        connections.remove(id)
    }

    /// Get all active connections count
    pub fn active_count(&self) -> usize {
        let connections = self.connections.read();
        connections
            .values()
            .filter(|c| c.state == ConnectionState::Connected)
            .count()
    }

    /// List all active connection IDs
    pub fn active_connections(&self) -> Vec<ConnectionId> {
        let connections = self.connections.read();
        connections
            .values()
            .filter(|c| c.state == ConnectionState::Connected)
            .map(|c| c.id)
            .collect()
    }
}

/// Application state with connection management
#[derive(Clone)]
pub struct RelayState {
    pub connection_registry: Arc<ConnectionRegistry>,
    pub log_tx: broadcast::Sender<String>,
}

impl RelayState {
    /// Create a new relay state
    pub fn new(log_tx: broadcast::Sender<String>) -> Self {
        Self {
            connection_registry: Arc::new(ConnectionRegistry::new()),
            log_tx,
        }
    }
}

/// Validate token from query
pub fn validate_token(token: Option<String>, valid_token: &str) -> Result<String, (StatusCode, String)> {
    let token = match token {
        Some(t) if !t.is_empty() => t,
        _ => {
            tracing::warn!("WebSocket connection rejected: missing token");
            return Err((StatusCode::UNAUTHORIZED, "Missing authentication token".to_string()));
        }
    };

    if token != valid_token {
        tracing::warn!("WebSocket connection rejected: invalid token");
        return Err((StatusCode::UNAUTHORIZED, "Invalid authentication token".to_string()));
    }

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_new() {
        let conn = Connection::new();
        assert_eq!(conn.state, ConnectionState::Connecting);
        assert!(conn.id != Uuid::nil());
    }

    #[test]
    fn test_connection_state_transitions() {
        let mut conn = Connection::new();
        
        // Connect
        conn.connect();
        assert_eq!(conn.state, ConnectionState::Connected);
        
        // Close
        conn.close();
        assert_eq!(conn.state, ConnectionState::Closing);
        
        // Disconnect
        conn.disconnect();
        assert_eq!(conn.state, ConnectionState::Disconnected);
    }

    #[test]
    fn test_connection_registry_register() {
        let registry = ConnectionRegistry::new();
        let conn = Connection::new();
        let id = conn.id;
        
        let registered_id = registry.register(conn);
        assert_eq!(registered_id, id);
        
        let retrieved = registry.get(&id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, id);
    }

    #[test]
    fn test_connection_registry_remove() {
        let registry = ConnectionRegistry::new();
        let conn = Connection::new();
        let id = conn.id;
        
        registry.register(conn);
        let removed = registry.remove(&id);
        assert!(removed.is_some());
        
        let retrieved = registry.get(&id);
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_connection_registry_active_count() {
        let registry = ConnectionRegistry::new();
        
        let mut conn1 = Connection::new();
        conn1.connect();
        registry.register(conn1);
        
        let mut conn2 = Connection::new();
        conn2.connect();
        registry.register(conn2);
        
        let conn3 = Connection::new();
        // Not connected
        registry.register(conn3);
        
        assert_eq!(registry.active_count(), 2);
    }

    #[test]
    fn test_validate_token_valid() {
        let result = validate_token(Some("valid-token".to_string()), "valid-token");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "valid-token");
    }

    #[test]
    fn test_validate_token_missing() {
        let result = validate_token(None, "valid-token");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().0, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_validate_token_empty() {
        let result = validate_token(Some("".to_string()), "valid-token");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().0, StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_validate_token_invalid() {
        let result = validate_token(Some("wrong-token".to_string()), "valid-token");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().0, StatusCode::UNAUTHORIZED);
    }
}
