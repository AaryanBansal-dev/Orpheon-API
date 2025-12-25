//! Temporal state capabilities.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::store::StateEntry;

/// A point-in-time snapshot of the state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Unique ID for this snapshot.
    pub id: Uuid,
    
    /// Version of the state at snapshot time.
    pub version: u64,
    
    /// Timestamp when the snapshot was taken.
    pub timestamp: DateTime<Utc>,
    
    /// All entries at snapshot time.
    pub entries: HashMap<String, StateEntry>,
}

impl StateSnapshot {
    /// Get a value from the snapshot.
    pub fn get(&self, key: &str) -> Option<&StateEntry> {
        self.entries.get(key)
    }
    
    /// Get all keys in the snapshot.
    pub fn keys(&self) -> Vec<&String> {
        self.entries.keys().collect()
    }
    
    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Query for time-travel operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeTravelQuery {
    /// The point in time to query.
    pub as_of: QueryTime,
    
    /// Keys to retrieve (None = all keys).
    pub keys: Option<Vec<String>>,
    
    /// Key prefix filter.
    pub prefix: Option<String>,
}

/// Specification for a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QueryTime {
    /// Absolute timestamp.
    Timestamp(DateTime<Utc>),
    
    /// Relative offset from now (in seconds, negative = past).
    Offset(i64),
    
    /// Specific version number.
    Version(u64),
}

impl QueryTime {
    /// Resolve to an absolute timestamp.
    pub fn resolve(&self) -> DateTime<Utc> {
        match self {
            QueryTime::Timestamp(ts) => *ts,
            QueryTime::Offset(secs) => {
                Utc::now() + chrono::Duration::seconds(*secs)
            }
            QueryTime::Version(_) => {
                // Version-based queries need store context
                Utc::now()
            }
        }
    }
}

/// Result of a simulation (speculative execution).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// ID of the simulation.
    pub id: Uuid,
    
    /// Initial state snapshot.
    pub initial_state: StateSnapshot,
    
    /// Final state after simulation.
    pub final_state: StateSnapshot,
    
    /// Changes that would occur.
    pub changes: Vec<SimulatedChange>,
    
    /// Whether the simulation succeeded.
    pub success: bool,
    
    /// Error message if failed.
    pub error: Option<String>,
    
    /// Simulated duration in milliseconds.
    pub simulated_duration_ms: u64,
    
    /// Simulated cost.
    pub simulated_cost: f64,
}

/// A change that would occur during simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedChange {
    /// Step in the simulation.
    pub step: u32,
    
    /// Key affected.
    pub key: String,
    
    /// Old value.
    pub old_value: Option<serde_json::Value>,
    
    /// New value.
    pub new_value: Option<serde_json::Value>,
    
    /// Timestamp in the simulation.
    pub timestamp: DateTime<Utc>,
}

/// State fork for copy-on-write branching.
#[derive(Debug, Clone)]
pub struct StateFork {
    /// Unique ID for this fork.
    pub id: Uuid,
    
    /// Name of the fork.
    pub name: String,
    
    /// When the fork was created.
    pub created_at: DateTime<Utc>,
    
    /// Parent fork ID (None = main state).
    pub parent_id: Option<Uuid>,
    
    /// Fork-specific state changes.
    pub changes: HashMap<String, StateEntry>,
}

impl StateFork {
    /// Create a new fork.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            created_at: Utc::now(),
            parent_id: None,
            changes: HashMap::new(),
        }
    }
    
    /// Create a child fork.
    pub fn child(&self, name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            created_at: Utc::now(),
            parent_id: Some(self.id),
            changes: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_time_offset() {
        let query = QueryTime::Offset(-3600); // 1 hour ago
        let timestamp = query.resolve();
        
        let now = Utc::now();
        let diff = (now - timestamp).num_seconds();
        
        // Should be approximately 1 hour
        assert!(diff > 3590 && diff < 3610);
    }

    #[test]
    fn test_snapshot() {
        let mut entries = HashMap::new();
        entries.insert(
            "key1".to_string(),
            StateEntry {
                key: "key1".to_string(),
                value: serde_json::json!("value1"),
                version: 1,
                timestamp: Utc::now(),
                deleted: false,
                metadata: HashMap::new(),
            },
        );
        
        let snapshot = StateSnapshot {
            id: Uuid::new_v4(),
            version: 1,
            timestamp: Utc::now(),
            entries,
        };
        
        assert_eq!(snapshot.len(), 1);
        assert!(snapshot.get("key1").is_some());
    }

    #[test]
    fn test_fork_hierarchy() {
        let parent = StateFork::new("main");
        let child = parent.child("feature-branch");
        
        assert_eq!(child.parent_id, Some(parent.id));
    }
}
