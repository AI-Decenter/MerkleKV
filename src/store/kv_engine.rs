//! # Key-Value Storage Engine
//!
//! This module provides the core storage functionality for MerkleKV.
//! Implements a persistent storage engine using Sled embedded database.
//!
//! ## Current Implementation
//! 
//! The current implementation is a persistent store that:
//! - Uses Sled embedded database for disk persistence
//! - Provides ACID transactions and durability guarantees
//! - Provides basic get/set/delete operations
//! - Returns all keys for iteration
//! - Survives process restarts and system crashes
//!
//! ## Features
//! 
//! - **Persistence**: All data is stored on disk and survives restarts
//! - **ACID Transactions**: Sled provides atomic, consistent, isolated, durable operations
//! - **Write-Ahead Logging**: Built into Sled for durability
//! - **Crash Recovery**: Automatic recovery on startup
//! - **Thread Safety**: Sled handles concurrent access safely

use anyhow::Result;
use sled::Db;
use std::path::Path;

/// Persistent key-value storage engine using Sled.
/// 
/// This storage implementation provides persistent storage using the Sled embedded database.
/// All data is written to disk and survives process restarts. Sled provides ACID transactions,
/// write-ahead logging, and automatic crash recovery.
/// 
/// **Features**:
/// - Persistent storage with automatic durability
/// - ACID transaction support
/// - Built-in write-ahead logging (WAL)
/// - Automatic crash recovery
/// - Thread-safe concurrent access
#[derive(Clone)]
pub struct KvEngine {
    /// Sled database instance for persistent storage
    db: Db,
}

impl KvEngine {
    /// Create a new storage engine instance.
    /// 
    /// # Arguments
    /// * `storage_path` - Path where data should be stored
    /// 
    /// # Returns
    /// * `Result<KvEngine>` - New storage engine instance or error
    /// 
    /// # Behavior
    /// Opens or creates a Sled database at the specified path. If the database
    /// already exists, it will be opened and any existing data will be available.
    /// The database provides automatic crash recovery and durability guarantees.
    /// 
    /// # Errors
    /// Returns an error if the database cannot be opened due to permission issues,
    /// disk space, or other I/O problems.
    pub fn new(storage_path: &str) -> Result<Self> {
        // Create the storage directory if it doesn't exist
        if let Some(parent) = Path::new(storage_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        // Open or create the Sled database
        let db = sled::open(storage_path)?;
        
        Ok(Self { db })
    }

    /// Retrieve a value by its key.
    /// 
    /// # Arguments
    /// * `key` - The key to look up
    /// 
    /// # Returns
    /// * `Option<String>` - The value if found, None otherwise
    /// 
    /// # Example
    /// ```rust
    /// let engine = KvEngine::new("./data")?;
    /// if let Some(value) = engine.get("user:123") {
    ///     println!("Found user: {}", value);
    /// }
    /// ```
    pub fn get(&self, key: &str) -> Option<String> {
        match self.db.get(key.as_bytes()) {
            Ok(Some(value)) => String::from_utf8(value.to_vec()).ok(),
            _ => None,
        }
    }

    /// Store a key-value pair.
    /// 
    /// This operation writes the data to disk immediately with durability guarantees.
    /// The write is atomic and either succeeds completely or fails without partial writes.
    /// 
    /// # Arguments
    /// * `key` - The key to store
    /// * `value` - The value to associate with the key
    /// 
    /// # Example
    /// ```rust
    /// let mut engine = KvEngine::new("./data")?;
    /// engine.set("user:123".to_string(), "john_doe".to_string());
    /// ```
    /// 
    /// # Performance Note
    /// Each write is immediately persisted to disk with WAL for durability.
    /// For high-throughput scenarios, consider batching operations.
    pub fn set(&mut self, key: String, value: String) {
        // Sled handles all the persistence, WAL, and durability automatically
        // The operation is atomic and durable
        if let Err(e) = self.db.insert(key.as_bytes(), value.as_bytes()) {
            log::error!("Failed to set key {}: {}", key, e);
        }
    }

    /// Delete a key-value pair.
    /// 
    /// Removes the key and its associated value from the database permanently.
    /// The operation is atomic and durable.
    /// 
    /// # Arguments
    /// * `key` - The key to delete
    /// 
    /// # Example
    /// ```rust
    /// let mut engine = KvEngine::new("./data")?;
    /// engine.delete("user:123");
    /// ```
    pub fn delete(&mut self, key: &str) {
        if let Err(e) = self.db.remove(key.as_bytes()) {
            log::error!("Failed to delete key {}: {}", key, e);
        }
    }

    /// Get all keys currently stored in the engine.
    /// 
    /// This is primarily used by the Merkle tree to rebuild its state
    /// and for debugging purposes.
    /// 
    /// # Returns
    /// * `Vec<String>` - Vector of all keys in the store
    /// 
    /// # Performance Note
    /// This operation scans all keys in the database. For large datasets,
    /// consider using iteration methods instead.
    pub fn keys(&self) -> Vec<String> {
        self.db
            .iter()
            .filter_map(|result| {
                result.ok().and_then(|(key, _)| {
                    String::from_utf8(key.to_vec()).ok()
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_kv_operations() {
        // Use a temporary directory for testing
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("test_db");
        let storage_path_str = storage_path.to_str().unwrap();
        
        let mut engine = KvEngine::new(storage_path_str).unwrap();
        
        // Test basic set and get operations
        engine.set("key1".to_string(), "value1".to_string());
        assert_eq!(engine.get("key1"), Some("value1".to_string()));
        
        // Test overwriting an existing key
        engine.set("key1".to_string(), "new_value".to_string());
        assert_eq!(engine.get("key1"), Some("new_value".to_string()));
        
        // Test delete operation
        engine.delete("key1");
        assert_eq!(engine.get("key1"), None);
        
        // Test keys() method with multiple entries
        engine.set("key2".to_string(), "value2".to_string());
        engine.set("key3".to_string(), "value3".to_string());
        
        let keys = engine.keys();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key2".to_string()));
        assert!(keys.contains(&"key3".to_string()));
    }

    #[test]
    fn test_persistence() {
        // Use a temporary directory for testing
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("persistence_test_db");
        let storage_path_str = storage_path.to_str().unwrap();
        
        // Create first engine instance and store some data
        {
            let mut engine = KvEngine::new(storage_path_str).unwrap();
            engine.set("persistent_key".to_string(), "persistent_value".to_string());
            engine.set("key2".to_string(), "value2".to_string());
        } // engine goes out of scope, simulating process termination
        
        // Create second engine instance and verify data persisted
        {
            let engine = KvEngine::new(storage_path_str).unwrap();
            assert_eq!(engine.get("persistent_key"), Some("persistent_value".to_string()));
            assert_eq!(engine.get("key2"), Some("value2".to_string()));
            assert_eq!(engine.get("nonexistent"), None);
            
            let keys = engine.keys();
            assert_eq!(keys.len(), 2);
            assert!(keys.contains(&"persistent_key".to_string()));
            assert!(keys.contains(&"key2".to_string()));
        }
    }

    #[test]
    fn test_empty_database() {
        // Test that a new database starts empty
        let temp_dir = tempdir().unwrap();
        let storage_path = temp_dir.path().join("empty_test_db");
        let storage_path_str = storage_path.to_str().unwrap();
        
        let engine = KvEngine::new(storage_path_str).unwrap();
        assert_eq!(engine.keys().len(), 0);
        assert_eq!(engine.get("any_key"), None);
    }
}
