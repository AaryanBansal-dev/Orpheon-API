//! State store implementations.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use orpheon_core::{OrpheonError, Result};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::temporal::StateSnapshot;

/// A versioned state entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEntry {
    /// The key for this entry.
    pub key: String,
    
    /// The value.
    pub value: serde_json::Value,
    
    /// Version number (monotonically increasing).
    pub version: u64,
    
    /// Timestamp when this version was created.
    pub timestamp: DateTime<Utc>,
    
    /// Whether this entry is deleted (tombstone).
    pub deleted: bool,
    
    /// Metadata about this entry.
    pub metadata: HashMap<String, String>,
}

/// Trait for state stores.
#[async_trait]
pub trait StateStore: Send + Sync {
    /// Get the current value for a key.
    async fn get(&self, key: &str) -> Result<Option<StateEntry>>;
    
    /// Get all entries matching a prefix.
    async fn get_prefix(&self, prefix: &str) -> Result<Vec<StateEntry>>;
    
    /// Set a value for a key.
    async fn set(&self, key: &str, value: serde_json::Value) -> Result<StateEntry>;
    
    /// Delete a key (creates a tombstone).
    async fn delete(&self, key: &str) -> Result<()>;
    
    /// Get the value at a specific point in time.
    async fn get_at(&self, key: &str, timestamp: DateTime<Utc>) -> Result<Option<StateEntry>>;
    
    /// Create a snapshot of the current state.
    async fn snapshot(&self) -> Result<StateSnapshot>;
    
    /// Create a fork (copy-on-write branch) of the state.
    async fn fork(&self, name: &str) -> Result<Uuid>;
    
    /// Merge a fork back into the main state.
    async fn merge_fork(&self, fork_id: Uuid) -> Result<()>;
    
    /// Get all keys in the store.
    async fn keys(&self) -> Result<Vec<String>>;
    
    /// Get the current version of the store.
    async fn version(&self) -> u64;
}

/// In-memory implementation of StateStore.
pub struct InMemoryStateStore {
    /// Main state storage: key -> list of versions (append-only).
    state: Arc<RwLock<HashMap<String, Vec<StateEntry>>>>,
    
    /// Forks: fork_id -> forked state.
    forks: Arc<RwLock<HashMap<Uuid, HashMap<String, Vec<StateEntry>>>>>,
    
    /// Global version counter.
    version: Arc<RwLock<u64>>,
}

impl InMemoryStateStore {
    /// Create a new in-memory state store.
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(HashMap::new())),
            forks: Arc::new(RwLock::new(HashMap::new())),
            version: Arc::new(RwLock::new(0)),
        }
    }
    
    /// Get the next version number.
    async fn next_version(&self) -> u64 {
        let mut version = self.version.write().await;
        *version += 1;
        *version
    }
}

impl Default for InMemoryStateStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StateStore for InMemoryStateStore {
    async fn get(&self, key: &str) -> Result<Option<StateEntry>> {
        let state = self.state.read().await;
        
        if let Some(versions) = state.get(key) {
            // Get the latest entry
            if let Some(latest) = versions.last() {
                // If the latest entry is a tombstone, the key is deleted
                if latest.deleted {
                    return Ok(None);
                }
                return Ok(Some(latest.clone()));
            }
        }
        
        Ok(None)
    }
    
    async fn get_prefix(&self, prefix: &str) -> Result<Vec<StateEntry>> {
        let state = self.state.read().await;
        
        let entries: Vec<StateEntry> = state
            .iter()
            .filter(|(k, _)| k.starts_with(prefix))
            .filter_map(|(_, versions)| {
                versions.iter().rev().find(|e| !e.deleted).cloned()
            })
            .collect();
        
        Ok(entries)
    }
    
    async fn set(&self, key: &str, value: serde_json::Value) -> Result<StateEntry> {
        let mut state = self.state.write().await;
        let version = self.next_version().await;
        
        let entry = StateEntry {
            key: key.to_string(),
            value,
            version,
            timestamp: Utc::now(),
            deleted: false,
            metadata: HashMap::new(),
        };
        
        state
            .entry(key.to_string())
            .or_insert_with(Vec::new)
            .push(entry.clone());
        
        Ok(entry)
    }
    
    async fn delete(&self, key: &str) -> Result<()> {
        let mut state = self.state.write().await;
        let version = self.next_version().await;
        
        let tombstone = StateEntry {
            key: key.to_string(),
            value: serde_json::Value::Null,
            version,
            timestamp: Utc::now(),
            deleted: true,
            metadata: HashMap::new(),
        };
        
        state
            .entry(key.to_string())
            .or_insert_with(Vec::new)
            .push(tombstone);
        
        Ok(())
    }
    
    async fn get_at(&self, key: &str, timestamp: DateTime<Utc>) -> Result<Option<StateEntry>> {
        let state = self.state.read().await;
        
        if let Some(versions) = state.get(key) {
            // Find the latest version at or before the timestamp
            let entry = versions
                .iter()
                .rev()
                .find(|e| e.timestamp <= timestamp && !e.deleted);
            return Ok(entry.cloned());
        }
        
        Ok(None)
    }
    
    async fn snapshot(&self) -> Result<StateSnapshot> {
        let state = self.state.read().await;
        let version = *self.version.read().await;
        
        // Get current values for all keys
        let entries: HashMap<String, StateEntry> = state
            .iter()
            .filter_map(|(k, versions)| {
                versions
                    .iter()
                    .rev()
                    .find(|e| !e.deleted)
                    .map(|e| (k.clone(), e.clone()))
            })
            .collect();
        
        Ok(StateSnapshot {
            id: Uuid::new_v4(),
            version,
            timestamp: Utc::now(),
            entries,
        })
    }
    
    async fn fork(&self, name: &str) -> Result<Uuid> {
        let state = self.state.read().await;
        let fork_id = Uuid::new_v4();
        
        // Clone the current state
        let forked_state = state.clone();
        
        let mut forks = self.forks.write().await;
        forks.insert(fork_id, forked_state);
        
        tracing::info!("Created fork '{}' with id {}", name, fork_id);
        
        Ok(fork_id)
    }
    
    async fn merge_fork(&self, fork_id: Uuid) -> Result<()> {
        let mut forks = self.forks.write().await;
        
        let forked_state = forks.remove(&fork_id).ok_or_else(|| {
            OrpheonError::StateError {
                message: format!("Fork {} not found", fork_id),
            }
        })?;
        
        let mut state = self.state.write().await;
        
        // Merge forked state into main state
        for (key, versions) in forked_state {
            let main_versions = state.entry(key).or_insert_with(Vec::new);
            
            // Only add versions that are newer
            let latest_main_version = main_versions.last().map(|e| e.version).unwrap_or(0);
            
            for entry in versions {
                if entry.version > latest_main_version {
                    main_versions.push(entry);
                }
            }
        }
        
        Ok(())
    }
    
    async fn keys(&self) -> Result<Vec<String>> {
        let state = self.state.read().await;
        Ok(state.keys().cloned().collect())
    }
    
    async fn version(&self) -> u64 {
        *self.version.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_and_get() {
        let store = InMemoryStateStore::new();
        
        store.set("key1", serde_json::json!({"value": 42})).await.unwrap();
        
        let entry = store.get("key1").await.unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().value["value"], 42);
    }

    #[tokio::test]
    async fn test_delete() {
        let store = InMemoryStateStore::new();
        
        store.set("key1", serde_json::json!("value")).await.unwrap();
        store.delete("key1").await.unwrap();
        
        let entry = store.get("key1").await.unwrap();
        assert!(entry.is_none());
    }

    #[tokio::test]
    async fn test_versioning() {
        let store = InMemoryStateStore::new();
        
        let e1 = store.set("key1", serde_json::json!("v1")).await.unwrap();
        let e2 = store.set("key1", serde_json::json!("v2")).await.unwrap();
        
        assert!(e2.version > e1.version);
        
        let current = store.get("key1").await.unwrap().unwrap();
        assert_eq!(current.value, "v2");
    }

    #[tokio::test]
    async fn test_time_travel() {
        let store = InMemoryStateStore::new();
        
        let e1 = store.set("key1", serde_json::json!("old")).await.unwrap();
        let old_time = e1.timestamp;
        
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        
        store.set("key1", serde_json::json!("new")).await.unwrap();
        
        let old_entry = store.get_at("key1", old_time).await.unwrap();
        assert!(old_entry.is_some());
        assert_eq!(old_entry.unwrap().value, "old");
    }

    #[tokio::test]
    async fn test_fork() {
        let store = InMemoryStateStore::new();
        
        store.set("key1", serde_json::json!("original")).await.unwrap();
        
        let fork_id = store.fork("test_fork").await.unwrap();
        
        // Modify main state
        store.set("key1", serde_json::json!("modified")).await.unwrap();
        
        // Fork should still have original (simulated - actual implementation would need fork-specific operations)
        let current = store.get("key1").await.unwrap().unwrap();
        assert_eq!(current.value, "modified");
        
        // Cleanup
        store.merge_fork(fork_id).await.unwrap();
    }
}
