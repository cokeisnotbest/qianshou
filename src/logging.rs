//! SSE Log Streaming Module
//!
//! Provides structured logging with levels and SSE broadcast capabilities.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Log levels for filtering and categorization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        LogLevel::Info
    }
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(format!("Unknown log level: {}", s)),
        }
    }
}

/// A structured log entry with timestamp, level, and message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Timestamp in ISO 8601 format
    pub timestamp: DateTime<Utc>,
    /// Log level
    pub level: LogLevel,
    /// Log message
    pub message: String,
    /// Optional connection ID for connection events
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_id: Option<String>,
}

impl LogEntry {
    /// Create a new log entry
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            message: message.into(),
            connection_id: None,
        }
    }
    
    /// Create a log entry with connection ID
    pub fn with_connection(level: LogLevel, message: impl Into<String>, connection_id: impl Into<String>) -> Self {
        Self {
            timestamp: Utc::now(),
            level,
            message: message.into(),
            connection_id: Some(connection_id.into()),
        }
    }
    
    /// Serialize to JSON string
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// Log broadcaster for SSE subscriptions
#[derive(Clone)]
pub struct LogBroadcaster {
    tx: broadcast::Sender<LogEntry>,
}

impl Default for LogBroadcaster {
    fn default() -> Self {
        Self::new()
    }
}

impl LogBroadcaster {
    /// Create a new log broadcaster
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self { tx }
    }
    
    /// Get a subscriber for receiving log entries
    pub fn subscribe(&self) -> LogSubscriber {
        LogSubscriber {
            rx: self.tx.subscribe(),
        }
    }
    
    /// Broadcast a log entry to all subscribers
    pub fn broadcast(&self, entry: LogEntry) {
        let _ = self.tx.send(entry);
    }
    
    /// Log at debug level
    pub fn debug(&self, message: impl Into<String>) {
        self.broadcast(LogEntry::new(LogLevel::Debug, message));
    }
    
    /// Log at info level
    pub fn info(&self, message: impl Into<String>) {
        self.broadcast(LogEntry::new(LogLevel::Info, message));
    }
    
    /// Log at warn level
    pub fn warn(&self, message: impl Into<String>) {
        self.broadcast(LogEntry::new(LogLevel::Warn, message));
    }
    
    /// Log at error level
    pub fn error(&self, message: impl Into<String>) {
        self.broadcast(LogEntry::new(LogLevel::Error, message));
    }
    
    /// Log connection open event
    pub fn log_connection_open(&self, connection_id: &str) {
        self.broadcast(LogEntry::with_connection(
            LogLevel::Info,
            "Connection opened",
            connection_id,
        ));
    }
    
    /// Log connection close event
    pub fn log_connection_close(&self, connection_id: &str) {
        self.broadcast(LogEntry::with_connection(
            LogLevel::Info,
            "Connection closed",
            connection_id,
        ));
    }
}

/// Log subscriber for receiving log entries via SSE
#[derive(Debug)]
pub struct LogSubscriber {
    rx: broadcast::Receiver<LogEntry>,
}

impl LogSubscriber {
    /// Receive the next log entry
    pub async fn recv(&mut self) -> Result<LogEntry, broadcast::error::RecvError> {
        self.rx.recv().await
    }
    
    /// Try to receive the next log entry without waiting
    pub fn try_recv(&mut self) -> Result<LogEntry, broadcast::error::TryRecvError> {
        self.rx.try_recv()
    }
}

/// Filter logs by minimum level
pub fn should_include_log(log_level: &LogLevel, min_level: &LogLevel) -> bool {
    let log_priority = match log_level {
        LogLevel::Debug => 0,
        LogLevel::Info => 1,
        LogLevel::Warn => 2,
        LogLevel::Error => 3,
    };
    
    let min_priority = match min_level {
        LogLevel::Debug => 0,
        LogLevel::Info => 1,
        LogLevel::Warn => 2,
        LogLevel::Error => 3,
    };
    
    log_priority >= min_priority
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_log_level_default() {
        let level = LogLevel::default();
        assert_eq!(level, LogLevel::Info);
    }
    
    #[test]
    fn test_log_level_display() {
        assert_eq!(LogLevel::Debug.to_string(), "debug");
        assert_eq!(LogLevel::Info.to_string(), "info");
        assert_eq!(LogLevel::Warn.to_string(), "warn");
        assert_eq!(LogLevel::Error.to_string(), "error");
    }
    
    #[test]
    fn test_log_level_from_str() {
        assert_eq!("debug".parse::<LogLevel>().unwrap(), LogLevel::Debug);
        assert_eq!("info".parse::<LogLevel>().unwrap(), LogLevel::Info);
        assert_eq!("warn".parse::<LogLevel>().unwrap(), LogLevel::Warn);
        assert_eq!("error".parse::<LogLevel>().unwrap(), LogLevel::Error);
        assert_eq!("warning".parse::<LogLevel>().unwrap(), LogLevel::Warn);
    }
    
    #[test]
    fn test_log_level_from_str_invalid() {
        assert!("invalid".parse::<LogLevel>().is_err());
        assert!("".parse::<LogLevel>().is_err());
    }
    
    #[test]
    fn test_log_entry_new() {
        let entry = LogEntry::new(LogLevel::Info, "test message");
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.message, "test message");
        assert!(entry.connection_id.is_none());
    }
    
    #[test]
    fn test_log_entry_with_connection() {
        let entry = LogEntry::with_connection(LogLevel::Info, "test message", "conn-123");
        assert_eq!(entry.connection_id, Some("conn-123".to_string()));
    }
    
    #[test]
    fn test_log_entry_to_json() {
        let entry = LogEntry::new(LogLevel::Info, "test message");
        let json = entry.to_json();
        assert!(json.contains("test message"));
        assert!(json.contains("info"));
    }
    
    #[test]
    fn test_log_broadcaster_new() {
        let broadcaster = LogBroadcaster::new();
        let _sub = broadcaster.subscribe();
    }
    
    #[test]
    fn test_log_broadcaster_broadcast() {
        let broadcaster = LogBroadcaster::new();
        let mut sub = broadcaster.subscribe();
        
        broadcaster.broadcast(LogEntry::new(LogLevel::Info, "test"));
        
        let received = sub.try_recv();
        assert!(received.is_ok());
        assert_eq!(received.unwrap().message, "test");
    }
    
    #[test]
    fn test_log_broadcaster_levels() {
        let broadcaster = LogBroadcaster::new();
        
        // Create subscriber first
        let mut sub = broadcaster.subscribe();
        
        // Broadcast each level
        broadcaster.debug("debug message");
        broadcaster.info("info message");
        broadcaster.warn("warn message");
        broadcaster.error("error message");
        
        assert_eq!(sub.try_recv().unwrap().message, "debug message");
        assert_eq!(sub.try_recv().unwrap().message, "info message");
        assert_eq!(sub.try_recv().unwrap().message, "warn message");
        assert_eq!(sub.try_recv().unwrap().message, "error message");
    }
    
    #[test]
    fn test_log_connection_events() {
        let broadcaster = LogBroadcaster::new();
        
        // Create subscriber first
        let mut sub = broadcaster.subscribe();
        
        broadcaster.log_connection_open("conn-123");
        broadcaster.log_connection_close("conn-123");
        
        let open = sub.try_recv().unwrap();
        assert_eq!(open.message, "Connection opened");
        assert_eq!(open.connection_id, Some("conn-123".to_string()));
        
        let close = sub.try_recv().unwrap();
        assert_eq!(close.message, "Connection closed");
        assert_eq!(close.connection_id, Some("conn-123".to_string()));
    }
    
    #[test]
    fn test_should_include_log() {
        // Debug includes everything
        assert!(should_include_log(&LogLevel::Debug, &LogLevel::Debug));
        assert!(should_include_log(&LogLevel::Info, &LogLevel::Debug));
        assert!(should_include_log(&LogLevel::Warn, &LogLevel::Debug));
        assert!(should_include_log(&LogLevel::Error, &LogLevel::Debug));
        
        // Info includes info, warn, error but not debug
        assert!(!should_include_log(&LogLevel::Debug, &LogLevel::Info));
        assert!(should_include_log(&LogLevel::Info, &LogLevel::Info));
        assert!(should_include_log(&LogLevel::Warn, &LogLevel::Info));
        assert!(should_include_log(&LogLevel::Error, &LogLevel::Info));
        
        // Warn includes warn, error but not debug, info
        assert!(!should_include_log(&LogLevel::Debug, &LogLevel::Warn));
        assert!(!should_include_log(&LogLevel::Info, &LogLevel::Warn));
        assert!(should_include_log(&LogLevel::Warn, &LogLevel::Warn));
        assert!(should_include_log(&LogLevel::Error, &LogLevel::Warn));
        
        // Error includes only error
        assert!(!should_include_log(&LogLevel::Debug, &LogLevel::Error));
        assert!(!should_include_log(&LogLevel::Info, &LogLevel::Error));
        assert!(!should_include_log(&LogLevel::Warn, &LogLevel::Error));
        assert!(should_include_log(&LogLevel::Error, &LogLevel::Error));
    }
    
    #[tokio::test]
    async fn test_log_subscriber_recv() {
        let broadcaster = LogBroadcaster::new();
        let mut sub = broadcaster.subscribe();
        
        broadcaster.broadcast(LogEntry::new(LogLevel::Info, "test message"));
        
        let received = sub.recv().await;
        assert!(received.is_ok());
        assert_eq!(received.unwrap().message, "test message");
    }
}
