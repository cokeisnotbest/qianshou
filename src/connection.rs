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

/// Connection type - distinguishes between clients and agents
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    /// Client connection (initiates requests)
    Client,
    /// Agent connection (processes requests)
    Agent,
}

impl Default for ConnectionType {
    fn default() -> Self {
        ConnectionType::Client
    }
}

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
    pub connection_type: ConnectionType,
    /// Optional agent/client identifier
    pub remote_id: Option<String>,
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
            connection_type: ConnectionType::Client, // Default to client
            remote_id: None,
            state: ConnectionState::Connecting,
            created_at: now,
            last_activity: now,
        }
    }

    /// Create a new connection with specific type
    pub fn with_type(connection_type: ConnectionType) -> Self {
        let mut conn = Self::new();
        conn.connection_type = connection_type;
        conn
    }

    /// Set the remote ID (agent_id or client_id)
    pub fn set_remote_id(&mut self, id: impl Into<String>) {
        self.remote_id = Some(id.into());
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

    /// Get all active clients
    pub fn active_clients(&self) -> Vec<Connection> {
        let connections = self.connections.read();
        connections
            .values()
            .filter(|c| c.state == ConnectionState::Connected && c.connection_type == ConnectionType::Client)
            .cloned()
            .collect()
    }

    /// Get all active agents
    pub fn active_agents(&self) -> Vec<Connection> {
        let connections = self.connections.read();
        connections
            .values()
            .filter(|c| c.state == ConnectionState::Connected && c.connection_type == ConnectionType::Agent)
            .cloned()
            .collect()
    }

    /// Get connection by remote_id (agent_id or client_id)
    pub fn get_by_remote_id(&self, remote_id: &str) -> Option<Connection> {
        let connections = self.connections.read();
        connections
            .values()
            .find(|c| c.remote_id.as_deref() == Some(remote_id))
            .cloned()
    }

    /// Update connection type
    pub fn update_connection_type(&self, id: &ConnectionId, connection_type: ConnectionType) -> bool {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.get_mut(id) {
            conn.connection_type = connection_type;
            conn.last_activity = Utc::now();
            true
        } else {
            false
        }
    }

    /// Update connection remote_id
    pub fn update_remote_id(&self, id: &ConnectionId, remote_id: impl Into<String>) -> bool {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.get_mut(id) {
            conn.remote_id = Some(remote_id.into());
            conn.last_activity = Utc::now();
            true
        } else {
            false
        }
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

    // === Tests for Bidirectional Relay Logic (US-006) ===

    #[test]
    fn test_connection_with_type() {
        let conn = Connection::with_type(ConnectionType::Agent);
        assert_eq!(conn.connection_type, ConnectionType::Agent);
        assert_eq!(conn.state, ConnectionState::Connecting);
    }

    #[test]
    fn test_connection_set_remote_id() {
        let mut conn = Connection::new();
        conn.set_remote_id("my-agent");
        assert_eq!(conn.remote_id, Some("my-agent".to_string()));
    }

    #[test]
    fn test_connection_registry_update_connection_type() {
        let registry = ConnectionRegistry::new();
        
        // Register a client connection
        let mut conn = Connection::new();
        conn.connect();
        let id = registry.register(conn);
        
        // Initially it's a client
        let retrieved = registry.get(&id).unwrap();
        assert_eq!(retrieved.connection_type, ConnectionType::Client);
        
        // Update to agent
        let updated = registry.update_connection_type(&id, ConnectionType::Agent);
        assert!(updated);
        
        // Verify update
        let retrieved = registry.get(&id).unwrap();
        assert_eq!(retrieved.connection_type, ConnectionType::Agent);
    }

    #[test]
    fn test_connection_registry_update_remote_id() {
        let registry = ConnectionRegistry::new();
        
        let mut conn = Connection::new();
        conn.connect();
        let id = registry.register(conn);
        
        // Update remote_id
        let updated = registry.update_remote_id(&id, "test-agent");
        assert!(updated);
        
        // Verify update
        let retrieved = registry.get(&id).unwrap();
        assert_eq!(retrieved.remote_id, Some("test-agent".to_string()));
    }

    #[test]
    fn test_connection_registry_active_clients() {
        let registry = ConnectionRegistry::new();
        
        // Register a client
        let mut client = Connection::new();
        client.connect();
        registry.register(client);
        
        // Register an agent
        let mut agent = Connection::with_type(ConnectionType::Agent);
        agent.connect();
        registry.register(agent);
        
        // Register a disconnected connection
        let disconnected = Connection::new();
        registry.register(disconnected);
        
        let clients = registry.active_clients();
        assert_eq!(clients.len(), 1);
        assert_eq!(clients[0].connection_type, ConnectionType::Client);
    }

    #[test]
    fn test_connection_registry_active_agents() {
        let registry = ConnectionRegistry::new();
        
        // Register a client
        let mut client = Connection::new();
        client.connect();
        registry.register(client);
        
        // Register an agent
        let mut agent = Connection::with_type(ConnectionType::Agent);
        agent.connect();
        registry.register(agent);
        
        // Register another agent (not connected)
        let disconnected_agent = Connection::with_type(ConnectionType::Agent);
        registry.register(disconnected_agent);
        
        let agents = registry.active_agents();
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].connection_type, ConnectionType::Agent);
    }

    #[test]
    fn test_connection_registry_get_by_remote_id() {
        let registry = ConnectionRegistry::new();
        
        // Register an agent with remote_id
        let mut agent = Connection::with_type(ConnectionType::Agent);
        agent.connect();
        agent.set_remote_id("my-agent");
        let id = registry.register(agent);
        
        // Find by remote_id
        let found = registry.get_by_remote_id("my-agent");
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, id);
    }

    #[test]
    fn test_connection_registry_get_by_remote_id_not_found() {
        let registry = ConnectionRegistry::new();
        
        let found = registry.get_by_remote_id("nonexistent");
        assert!(found.is_none());
    }

    #[test]
    fn test_update_nonexistent_connection_fails() {
        let registry = ConnectionRegistry::new();
        
        let fake_id = Uuid::new_v4();
        let updated = registry.update_connection_type(&fake_id, ConnectionType::Agent);
        assert!(!updated);
        
        let updated = registry.update_remote_id(&fake_id, "test");
        assert!(!updated);
    }
}
